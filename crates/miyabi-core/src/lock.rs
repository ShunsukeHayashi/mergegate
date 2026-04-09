//! File lock manager for deterministic task execution.

use crate::error::{Error, Result};
use crate::store::{
    lease_expiry, EventStore, FileLockEntry, SnapshotStore, TaskEvent, TaskEventType,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaseConfig {
    pub lease_duration_sec: u64,
    pub heartbeat_interval_sec: u64,
    pub stale_after_missed_heartbeats: u64,
}

impl Default for LeaseConfig {
    fn default() -> Self {
        Self {
            lease_duration_sec: 300,
            heartbeat_interval_sec: 60,
            stale_after_missed_heartbeats: 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockConflict {
    pub conflicting: bool,
    pub held_by: Option<String>,
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaseSweep {
    pub released: Vec<String>,
    pub active: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FileLockManager {
    event_store: EventStore,
    snapshot_store: SnapshotStore,
    config: LeaseConfig,
}

impl FileLockManager {
    pub fn new(
        event_store: EventStore,
        snapshot_store: SnapshotStore,
        config: LeaseConfig,
    ) -> Self {
        Self {
            event_store,
            snapshot_store,
            config,
        }
    }

    pub fn acquire_lock(
        &self,
        task_id: &str,
        agent: &str,
        node: &str,
        files: &[String],
    ) -> Result<()> {
        let mut snapshot = self.snapshot_store.load()?;
        self.purge_stale_from_snapshot(&mut snapshot, Utc::now());

        if snapshot.get_task(task_id).is_none() {
            return Err(Error::Validation(format!("unknown task: {task_id}")));
        }

        let conflict = find_conflict(&snapshot.file_locks, task_id, files);
        if let Some(entry) = conflict {
            return Err(Error::Validation(format!(
                "lock conflict: {} held by {}@{}",
                entry.task_id, entry.agent, entry.node
            )));
        }

        let now = Utc::now();
        let expires_at = self.compute_expires_at(now);
        {
            let task = snapshot
                .get_task_mut(task_id)
                .ok_or_else(|| Error::Validation(format!("unknown task: {task_id}")))?;
            task.lock = Some(crate::store::TaskLockSnapshot {
                locked_by: format!("{agent}@{node}"),
                locked_at: now,
                lease_duration_sec: self.config.lease_duration_sec,
                last_heartbeat: now,
                affected_files: files.to_vec(),
            });
            task.updated_at = now;
        }

        for file in files {
            snapshot.file_locks.insert(
                file.clone(),
                FileLockEntry {
                    task_id: task_id.to_string(),
                    agent: agent.to_string(),
                    node: node.to_string(),
                    expires_at,
                },
            );
        }

        let expected_version = snapshot.version;
        self.snapshot_store.save(&snapshot, expected_version)?;
        self.event_store.append(&TaskEvent {
            id: format!("{task_id}-lock-acquired-{}", now.timestamp_millis()),
            ts: now,
            event_type: TaskEventType::LockAcquired,
            task_id: task_id.to_string(),
            agent: agent.to_string(),
            node: node.to_string(),
            payload: serde_json::json!({
                "files": files,
                "expires_at": expires_at,
                "lease_duration_sec": self.config.lease_duration_sec
            }),
            version: expected_version + 1,
        })?;

        Ok(())
    }

    pub fn renew_lease(&self, task_id: &str, agent: &str, node: &str) -> Result<()> {
        let mut snapshot = self.snapshot_store.load()?;
        let now = Utc::now();
        let expires_at = self.compute_expires_at(now);
        let expected_version = snapshot.version;
        let owner = format!("{agent}@{node}");
        let files = {
            let task = snapshot
                .get_task_mut(task_id)
                .ok_or_else(|| Error::Validation(format!("unknown task: {task_id}")))?;
            let lock = task
                .lock
                .as_mut()
                .ok_or_else(|| Error::Validation(format!("task {task_id} has no lock")))?;

            if lock.locked_by != owner {
                return Err(Error::PermissionDenied(format!(
                    "lock owned by {}, not {owner}",
                    lock.locked_by
                )));
            }

            lock.last_heartbeat = now;
            task.updated_at = now;
            lock.affected_files.clone()
        };

        for file in &files {
            if let Some(entry) = snapshot.file_locks.get_mut(file) {
                entry.expires_at = expires_at;
            }
        }

        self.snapshot_store.save(&snapshot, expected_version)?;
        self.event_store.append(&TaskEvent {
            id: format!("{task_id}-lock-heartbeat-{}", now.timestamp_millis()),
            ts: now,
            event_type: TaskEventType::LockHeartbeat,
            task_id: task_id.to_string(),
            agent: agent.to_string(),
            node: node.to_string(),
            payload: serde_json::json!({ "expires_at": expires_at }),
            version: expected_version + 1,
        })?;

        Ok(())
    }

    pub fn release_lock(&self, task_id: &str) -> Result<()> {
        let mut snapshot = self.snapshot_store.load()?;
        let now = Utc::now();
        let expected_version = snapshot.version;
        let Some(files) = ({
            let task = snapshot
                .get_task_mut(task_id)
                .ok_or_else(|| Error::Validation(format!("unknown task: {task_id}")))?;

            let files = task.lock.take().map(|lock| lock.affected_files);
            task.updated_at = now;
            files
        }) else {
            return Ok(());
        };

        for file in files {
            snapshot.file_locks.remove(&file);
        }

        self.snapshot_store.save(&snapshot, expected_version)?;
        self.event_store.append(&TaskEvent {
            id: format!("{task_id}-lock-released-{}", now.timestamp_millis()),
            ts: now,
            event_type: TaskEventType::LockReleased,
            task_id: task_id.to_string(),
            agent: "system".into(),
            node: "system".into(),
            payload: serde_json::json!({}),
            version: expected_version + 1,
        })?;

        Ok(())
    }

    pub fn release_expired_leases(&self) -> Result<LeaseSweep> {
        let mut snapshot = self.snapshot_store.load()?;
        let now = Utc::now();
        let mut released = Vec::new();
        let mut active = Vec::new();

        for task in snapshot.tasks.iter_mut() {
            let is_expired = task
                .lock
                .as_ref()
                .map(|lock| self.is_expired(lock.last_heartbeat, now))
                .unwrap_or(false);

            if is_expired {
                if let Some(lock) = task.lock.take() {
                    for file in lock.affected_files {
                        snapshot.file_locks.remove(&file);
                    }
                    released.push(task.id.clone());
                }
                task.updated_at = now;
            } else if task.lock.is_some() {
                active.push(task.id.clone());
            }
        }

        if !released.is_empty() {
            let expected_version = snapshot.version;
            self.snapshot_store.save(&snapshot, expected_version)?;
            for task_id in &released {
                self.event_store.append(&TaskEvent {
                    id: format!("{task_id}-lock-released-expired-{}", now.timestamp_millis()),
                    ts: now,
                    event_type: TaskEventType::LockReleased,
                    task_id: task_id.clone(),
                    agent: "system".into(),
                    node: "system".into(),
                    payload: serde_json::json!({ "forced": true, "reason": "lease_expired" }),
                    version: expected_version + 1,
                })?;
            }
        }

        Ok(LeaseSweep { released, active })
    }

    pub fn has_conflict(&self, files: &[String]) -> Result<LockConflict> {
        let mut snapshot = self.snapshot_store.load()?;
        self.purge_stale_from_snapshot(&mut snapshot, Utc::now());

        if let Some(entry) = find_conflict(&snapshot.file_locks, "", files) {
            return Ok(LockConflict {
                conflicting: true,
                held_by: Some(format!("{}@{}", entry.agent, entry.node)),
                task_id: Some(entry.task_id.clone()),
            });
        }

        Ok(LockConflict {
            conflicting: false,
            held_by: None,
            task_id: None,
        })
    }

    fn purge_stale_from_snapshot(
        &self,
        snapshot: &mut crate::store::TasksSnapshot,
        now: DateTime<Utc>,
    ) {
        let stale_task_ids: Vec<String> = snapshot
            .tasks
            .iter()
            .filter_map(|task| {
                task.lock.as_ref().and_then(|lock| {
                    if self.is_expired(lock.last_heartbeat, now) {
                        Some(task.id.clone())
                    } else {
                        None
                    }
                })
            })
            .collect();

        for task_id in stale_task_ids {
            if let Some(task) = snapshot.get_task_mut(&task_id) {
                if let Some(lock) = task.lock.take() {
                    for file in lock.affected_files {
                        snapshot.file_locks.remove(&file);
                    }
                }
            }
        }
    }

    fn is_expired(&self, last_heartbeat: DateTime<Utc>, now: DateTime<Utc>) -> bool {
        let max_age = self.config.lease_duration_sec as i64
            + (self.config.heartbeat_interval_sec * self.config.stale_after_missed_heartbeats)
                as i64;
        now > last_heartbeat + Duration::seconds(max_age)
    }

    fn compute_expires_at(&self, now: DateTime<Utc>) -> DateTime<Utc> {
        lease_expiry(
            now + Duration::seconds(
                (self.config.heartbeat_interval_sec * self.config.stale_after_missed_heartbeats)
                    as i64,
            ),
            self.config.lease_duration_sec,
        )
    }
}

fn find_conflict<'a>(
    file_locks: &'a std::collections::HashMap<String, FileLockEntry>,
    task_id: &str,
    files: &[String],
) -> Option<&'a FileLockEntry> {
    files.iter().find_map(|file| {
        file_locks.get(file).and_then(|entry| {
            if task_id.is_empty() || entry.task_id != task_id {
                Some(entry)
            } else {
                None
            }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{CompletionMode, ExecutionTask};
    use tempfile::TempDir;

    fn fixture() -> (TempDir, EventStore, SnapshotStore, FileLockManager) {
        let tmp = TempDir::new().unwrap();
        let event_store = EventStore::new(tmp.path().join("events.jsonl"));
        let snapshot_store = SnapshotStore::new(
            tmp.path().join("tasks.snapshot.json"),
            tmp.path().join(".tasks.lock"),
        );
        let manager = FileLockManager::new(
            event_store.clone(),
            snapshot_store.clone(),
            LeaseConfig {
                lease_duration_sec: 1,
                heartbeat_interval_sec: 1,
                stale_after_missed_heartbeats: 0,
            },
        );
        (tmp, event_store, snapshot_store, manager)
    }

    fn seed_task(snapshot_store: &SnapshotStore, id: &str) {
        let mut snapshot = snapshot_store.load().unwrap();
        let mut task = ExecutionTask::new(id, "Phase A");
        task.completion_mode = CompletionMode::GithubPr;
        snapshot.upsert_task(task);
        let version = snapshot.version;
        snapshot_store.save(&snapshot, version).unwrap();
    }

    #[test]
    fn acquires_detects_conflict_and_releases() {
        let (_tmp, _events, snapshot_store, manager) = fixture();
        seed_task(&snapshot_store, "task-a");
        seed_task(&snapshot_store, "task-b");

        manager
            .acquire_lock("task-a", "codex", "macbook", &[String::from("src/lib.rs")])
            .unwrap();
        let conflict = manager.has_conflict(&[String::from("src/lib.rs")]).unwrap();
        assert!(conflict.conflicting);

        let err = manager
            .acquire_lock("task-b", "codex", "macbook", &[String::from("src/lib.rs")])
            .unwrap_err();
        assert!(matches!(err, Error::Validation(_)));

        manager.release_lock("task-a").unwrap();
        let conflict = manager.has_conflict(&[String::from("src/lib.rs")]).unwrap();
        assert!(!conflict.conflicting);
    }

    #[test]
    fn renews_and_expires_leases() {
        let (_tmp, _events, snapshot_store, manager) = fixture();
        seed_task(&snapshot_store, "task-a");

        manager
            .acquire_lock("task-a", "codex", "macbook", &[String::from("src/lib.rs")])
            .unwrap();
        manager.renew_lease("task-a", "codex", "macbook").unwrap();

        {
            let mut snapshot = snapshot_store.load().unwrap();
            let task = snapshot.get_task_mut("task-a").unwrap();
            let lock = task.lock.as_mut().unwrap();
            lock.last_heartbeat = Utc::now() - Duration::seconds(5);
            let version = snapshot.version;
            snapshot_store.save(&snapshot, version).unwrap();
        }

        let sweep = manager.release_expired_leases().unwrap();
        assert_eq!(sweep.released, vec!["task-a".to_string()]);
        let snapshot = snapshot_store.load().unwrap();
        assert!(snapshot.get_task("task-a").unwrap().lock.is_none());
    }
}
