//! Deterministic task protocol storage primitives.

use crate::error::{Error, Result};
use chrono::{DateTime, Duration, Utc};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    Draft,
    Pending,
    Analyzing,
    Implementing,
    Reviewing,
    Merged,
    Deploying,
    Done,
    Blocked,
    Failed,
    Cancelled,
    AwaitingGithubSync,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletionMode {
    GithubPr,
    Manual,
    ExternalOp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ImpactRiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskImpact {
    pub risk_level: ImpactRiskLevel,
    pub affected_symbols: usize,
    pub depth1: Vec<String>,
    pub analyzed_at: DateTime<Utc>,
    pub analyzed_commit: Option<String>,
    pub input_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubEvidence {
    pub pr_number: u64,
    pub pr_head_ref: String,
    pub pr_state: GitHubPrState,
    pub merge_commit_sha: Option<String>,
    pub merged_at: Option<DateTime<Utc>>,
    pub review_decision: Option<ReviewDecision>,
    pub issue_state: GitHubIssueState,
    pub issue_closed_by_pr: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GitHubPrState {
    Open,
    Merged,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReviewDecision {
    Approved,
    ChangesRequested,
    ReviewRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GitHubIssueState {
    Open,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HumanApproval {
    pub required: bool,
    pub approved_by: Option<String>,
    pub approved_at: Option<DateTime<Utc>>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskLockSnapshot {
    pub locked_by: String,
    pub locked_at: DateTime<Utc>,
    pub lease_duration_sec: u64,
    pub last_heartbeat: DateTime<Utc>,
    pub affected_files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionTask {
    pub id: String,
    pub title: String,
    pub current_state: TaskState,
    pub dependencies: Vec<String>,
    pub dependents: Vec<String>,
    pub soft_dependencies: Vec<String>,
    pub lock: Option<TaskLockSnapshot>,
    pub impact: Option<TaskImpact>,
    pub branch_name: Option<String>,
    pub github_evidence: Option<GitHubEvidence>,
    pub completion_mode: CompletionMode,
    pub human_approval: Option<HumanApproval>,
    pub priority: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ExecutionTask {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            title: title.into(),
            current_state: TaskState::Draft,
            dependencies: Vec::new(),
            dependents: Vec::new(),
            soft_dependencies: Vec::new(),
            lock: None,
            impact: None,
            branch_name: None,
            github_evidence: None,
            completion_mode: CompletionMode::GithubPr,
            human_approval: None,
            priority: 0,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskEventType {
    StateTransition,
    LockAcquired,
    LockReleased,
    LockHeartbeat,
    DagChanged,
    GithubSynced,
    GatePassed,
    GateRejected,
    HumanApproved,
    ImpactRecorded,
    BranchCreated,
    PrCreated,
    MergeVerified,
    AuditRecorded,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskEvent {
    pub id: String,
    pub ts: DateTime<Utc>,
    #[serde(rename = "type")]
    pub event_type: TaskEventType,
    pub task_id: String,
    pub agent: String,
    pub node: String,
    pub payload: Value,
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileLockEntry {
    pub task_id: String,
    pub agent: String,
    pub node: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TasksSnapshot {
    pub version: u64,
    pub generated_at: DateTime<Utc>,
    pub generated_from_event_id: Option<String>,
    pub tasks: Vec<ExecutionTask>,
    pub file_locks: HashMap<String, FileLockEntry>,
}

impl Default for TasksSnapshot {
    fn default() -> Self {
        Self {
            version: 0,
            generated_at: Utc::now(),
            generated_from_event_id: None,
            tasks: Vec::new(),
            file_locks: HashMap::new(),
        }
    }
}

impl TasksSnapshot {
    pub fn get_task(&self, task_id: &str) -> Option<&ExecutionTask> {
        self.tasks.iter().find(|task| task.id == task_id)
    }

    pub fn get_task_mut(&mut self, task_id: &str) -> Option<&mut ExecutionTask> {
        self.tasks.iter_mut().find(|task| task.id == task_id)
    }

    pub fn upsert_task(&mut self, task: ExecutionTask) {
        if let Some(existing) = self.get_task_mut(&task.id) {
            *existing = task;
        } else {
            self.tasks.push(task);
        }
    }

    pub fn remove_task(&mut self, task_id: &str) -> Option<ExecutionTask> {
        let index = self.tasks.iter().position(|task| task.id == task_id)?;
        Some(self.tasks.remove(index))
    }
}

#[derive(Debug, Clone)]
pub struct EventStore {
    file_path: PathBuf,
}

impl EventStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            file_path: path.into(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.file_path
    }

    pub fn append(&self, event: &TaskEvent) -> Result<()> {
        ensure_parent_dir(&self.file_path)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)?;
        serde_json::to_writer(&mut file, event)?;
        writeln!(file)?;
        Ok(())
    }

    pub fn replay(&self, since_id: Option<&str>) -> Result<Vec<TaskEvent>> {
        if !self.file_path.exists() {
            return Ok(Vec::new());
        }

        let reader = BufReader::new(File::open(&self.file_path)?);
        let mut collecting = since_id.is_none();
        let mut events = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let event: TaskEvent = serde_json::from_str(&line)?;
            if !collecting {
                if Some(event.id.as_str()) == since_id {
                    collecting = true;
                }
                continue;
            }
            events.push(event);
        }

        Ok(events)
    }

    pub fn replay_for_task(&self, task_id: &str) -> Result<Vec<TaskEvent>> {
        Ok(self
            .replay(None)?
            .into_iter()
            .filter(|event| event.task_id == task_id)
            .collect())
    }
}

#[derive(Debug, Clone)]
pub struct SnapshotStore {
    file_path: PathBuf,
    lock_file_path: PathBuf,
}

impl SnapshotStore {
    pub fn new(file_path: impl Into<PathBuf>, lock_file_path: impl Into<PathBuf>) -> Self {
        Self {
            file_path: file_path.into(),
            lock_file_path: lock_file_path.into(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.file_path
    }

    pub fn load(&self) -> Result<TasksSnapshot> {
        if !self.file_path.exists() {
            return Ok(TasksSnapshot::default());
        }
        let raw = fs::read_to_string(&self.file_path)?;
        match serde_json::from_str(&raw) {
            Ok(snapshot) => Ok(snapshot),
            Err(_) => {
                let legacy: LegacyTasksFile = serde_json::from_str(&raw)?;
                Ok(legacy.into_snapshot())
            }
        }
    }

    pub fn save(&self, snapshot: &TasksSnapshot, expected_version: u64) -> Result<()> {
        ensure_parent_dir(&self.file_path)?;
        ensure_parent_dir(&self.lock_file_path)?;

        let lock_file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&self.lock_file_path)?;
        lock_file.lock_exclusive()?;

        let result = (|| -> Result<()> {
            let current = self.load()?;
            if current.version != expected_version {
                return Err(Error::Validation(format!(
                    "CAS conflict: expected v{expected_version}, got v{}",
                    current.version
                )));
            }

            let mut next = snapshot.clone();
            next.version = expected_version + 1;
            next.generated_at = Utc::now();

            let tmp_path = self.file_path.with_extension("json.tmp");
            fs::write(&tmp_path, serde_json::to_vec_pretty(&next)?)?;
            fs::rename(&tmp_path, &self.file_path)?;
            Ok(())
        })();

        let unlock_result = lock_file.unlock();
        result?;
        unlock_result?;
        Ok(())
    }

    pub fn rebuild(&self, event_store: &EventStore) -> Result<TasksSnapshot> {
        let mut snapshot = self.load()?;

        for event in event_store.replay(None)? {
            snapshot.generated_from_event_id = Some(event.id.clone());
            apply_event(&mut snapshot, &event)?;
        }

        Ok(snapshot)
    }
}

fn apply_event(snapshot: &mut TasksSnapshot, event: &TaskEvent) -> Result<()> {
    match event.event_type {
        TaskEventType::StateTransition => {
            let to = event
                .payload
                .get("to")
                .ok_or_else(|| Error::Validation("state_transition missing payload.to".into()))?;
            let state: TaskState = serde_json::from_value(to.clone())?;
            let task = snapshot
                .get_task_mut(&event.task_id)
                .ok_or_else(|| Error::Validation(format!("unknown task: {}", event.task_id)))?;
            task.current_state = state;
            task.updated_at = event.ts;
        }
        TaskEventType::LockAcquired => {
            let files: Vec<String> = serde_json::from_value(
                event
                    .payload
                    .get("files")
                    .cloned()
                    .ok_or_else(|| Error::Validation("lock_acquired missing files".into()))?,
            )?;
            let expires_at: DateTime<Utc> =
                serde_json::from_value(event.payload.get("expires_at").cloned().ok_or_else(
                    || Error::Validation("lock_acquired missing expires_at".into()),
                )?)?;
            let lease_duration_sec = event
                .payload
                .get("lease_duration_sec")
                .and_then(|v| v.as_u64())
                .unwrap_or(300);

            let task = snapshot
                .get_task_mut(&event.task_id)
                .ok_or_else(|| Error::Validation(format!("unknown task: {}", event.task_id)))?;
            task.lock = Some(TaskLockSnapshot {
                locked_by: format!("{}@{}", event.agent, event.node),
                locked_at: event.ts,
                lease_duration_sec,
                last_heartbeat: event.ts,
                affected_files: files.clone(),
            });
            task.updated_at = event.ts;

            for file in files {
                snapshot.file_locks.insert(
                    file,
                    FileLockEntry {
                        task_id: event.task_id.clone(),
                        agent: event.agent.clone(),
                        node: event.node.clone(),
                        expires_at,
                    },
                );
            }
        }
        TaskEventType::LockHeartbeat => {
            let expires_at: DateTime<Utc> =
                serde_json::from_value(event.payload.get("expires_at").cloned().ok_or_else(
                    || Error::Validation("lock_heartbeat missing expires_at".into()),
                )?)?;
            let affected_files = {
                let task = snapshot
                    .get_task_mut(&event.task_id)
                    .ok_or_else(|| Error::Validation(format!("unknown task: {}", event.task_id)))?;
                let affected_files = if let Some(lock) = &mut task.lock {
                    lock.last_heartbeat = event.ts;
                    lock.affected_files.clone()
                } else {
                    Vec::new()
                };
                task.updated_at = event.ts;
                affected_files
            };

            for file in affected_files {
                if let Some(entry) = snapshot.file_locks.get_mut(&file) {
                    entry.expires_at = expires_at;
                }
            }
        }
        TaskEventType::LockReleased => {
            if let Some(files_to_remove) = {
                if let Some(task) = snapshot.get_task_mut(&event.task_id) {
                    let files = task.lock.take().map(|lock| lock.affected_files);
                    task.updated_at = event.ts;
                    files
                } else {
                    None
                }
            } {
                for file in files_to_remove {
                    snapshot.file_locks.remove(&file);
                }
            }
        }
        _ => {}
    }

    snapshot.version = event.version;
    Ok(())
}

fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

#[derive(Debug, Clone, Deserialize)]
struct LegacyTasksFile {
    #[serde(default)]
    version: u64,
    #[serde(default)]
    tasks: Vec<LegacyTask>,
}

impl LegacyTasksFile {
    fn into_snapshot(self) -> TasksSnapshot {
        let mut snapshot = TasksSnapshot {
            version: self.version,
            generated_at: Utc::now(),
            generated_from_event_id: None,
            tasks: self.tasks.into_iter().map(LegacyTask::into_execution_task).collect(),
            file_locks: HashMap::new(),
        };

        for task in &snapshot.tasks {
            if let Some(lock) = &task.lock {
                let owner_parts: Vec<&str> = lock.locked_by.split('@').collect();
                let agent = owner_parts.first().copied().unwrap_or("unknown").to_string();
                let node = owner_parts.get(1).copied().unwrap_or("unknown").to_string();
                let expires_at = lease_expiry(lock.last_heartbeat, lock.lease_duration_sec);

                for file in &lock.affected_files {
                    snapshot.file_locks.insert(
                        file.clone(),
                        FileLockEntry {
                            task_id: task.id.clone(),
                            agent: agent.clone(),
                            node: node.clone(),
                            expires_at,
                        },
                    );
                }
            }
        }

        snapshot
    }
}

#[derive(Debug, Clone, Deserialize)]
struct LegacyTask {
    id: String,
    title: String,
    #[serde(default)]
    state: String,
    #[serde(default)]
    dependencies: Vec<String>,
    #[serde(default)]
    dependents: Vec<String>,
    #[serde(default)]
    soft_dependencies: Vec<String>,
    #[serde(default)]
    lock: Option<LegacyLock>,
    #[serde(default)]
    impact: Option<LegacyImpact>,
    #[serde(default)]
    branch_name: Option<String>,
    #[serde(default)]
    pr_number: Option<u64>,
    #[serde(default)]
    merge_commit: Option<String>,
    #[serde(default)]
    created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    updated_at: Option<DateTime<Utc>>,
}

impl LegacyTask {
    fn into_execution_task(self) -> ExecutionTask {
        let created_at = self.created_at.unwrap_or_else(Utc::now);
        let updated_at = self.updated_at.unwrap_or(created_at);
        let github_evidence = self.pr_number.map(|pr_number| GitHubEvidence {
            pr_number,
            pr_head_ref: self.branch_name.clone().unwrap_or_default(),
            pr_state: if self.merge_commit.is_some() {
                GitHubPrState::Merged
            } else {
                GitHubPrState::Open
            },
            merge_commit_sha: self.merge_commit,
            merged_at: None,
            review_decision: None,
            issue_state: GitHubIssueState::Open,
            issue_closed_by_pr: false,
        });

        ExecutionTask {
            id: self.id,
            title: self.title,
            current_state: legacy_state(&self.state),
            dependencies: self.dependencies,
            dependents: self.dependents,
            soft_dependencies: self.soft_dependencies,
            lock: self.lock.map(LegacyLock::into_snapshot),
            impact: self.impact.map(LegacyImpact::into_snapshot),
            branch_name: self.branch_name,
            github_evidence,
            completion_mode: CompletionMode::GithubPr,
            human_approval: None,
            priority: 0,
            created_at,
            updated_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct LegacyLock {
    locked_by: String,
    locked_at: DateTime<Utc>,
    ttl_secs: u64,
    #[serde(default)]
    affected_files: Vec<String>,
}

impl LegacyLock {
    fn into_snapshot(self) -> TaskLockSnapshot {
        TaskLockSnapshot {
            locked_by: self.locked_by,
            locked_at: self.locked_at,
            lease_duration_sec: self.ttl_secs,
            last_heartbeat: self.locked_at,
            affected_files: self.affected_files,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct LegacyImpact {
    risk_level: ImpactRiskLevel,
    affected_symbols: usize,
    #[serde(default)]
    depth1: Vec<String>,
    analyzed_at: DateTime<Utc>,
}

impl LegacyImpact {
    fn into_snapshot(self) -> TaskImpact {
        TaskImpact {
            risk_level: self.risk_level,
            affected_symbols: self.affected_symbols,
            depth1: self.depth1,
            analyzed_at: self.analyzed_at,
            analyzed_commit: None,
            input_hash: None,
        }
    }
}

fn legacy_state(state: &str) -> TaskState {
    match state {
        "draft" => TaskState::Draft,
        "pending" => TaskState::Pending,
        "analyzing" => TaskState::Analyzing,
        "implementing" => TaskState::Implementing,
        "reviewing" => TaskState::Reviewing,
        "merged" => TaskState::Merged,
        "deploying" => TaskState::Deploying,
        "done" => TaskState::Done,
        "blocked" => TaskState::Blocked,
        "failed" => TaskState::Failed,
        "cancelled" => TaskState::Cancelled,
        "awaiting_github_sync" => TaskState::AwaitingGithubSync,
        _ => TaskState::Draft,
    }
}

pub fn lease_expiry(last_heartbeat: DateTime<Utc>, lease_duration_sec: u64) -> DateTime<Utc> {
    last_heartbeat + Duration::seconds(lease_duration_sec as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_task(id: &str) -> ExecutionTask {
        ExecutionTask::new(id, format!("Task {id}"))
    }

    #[test]
    fn event_store_appends_and_filters_by_task() {
        let tmp = TempDir::new().unwrap();
        let store = EventStore::new(tmp.path().join("events.jsonl"));

        let event_a = TaskEvent {
            id: "1".into(),
            ts: Utc::now(),
            event_type: TaskEventType::GatePassed,
            task_id: "task-a".into(),
            agent: "codex".into(),
            node: "macbook".into(),
            payload: serde_json::json!({"gate": "gate_0"}),
            version: 1,
        };
        let event_b = TaskEvent {
            id: "2".into(),
            task_id: "task-b".into(),
            ..event_a.clone()
        };

        store.append(&event_a).unwrap();
        store.append(&event_b).unwrap();

        let filtered = store.replay_for_task("task-a").unwrap();
        assert_eq!(filtered, vec![event_a]);
    }

    #[test]
    fn snapshot_store_enforces_cas() {
        let tmp = TempDir::new().unwrap();
        let store = SnapshotStore::new(
            tmp.path().join("tasks.snapshot.json"),
            tmp.path().join(".tasks.lock"),
        );

        let mut snapshot = TasksSnapshot::default();
        snapshot.upsert_task(sample_task("task-a"));
        store.save(&snapshot, 0).unwrap();

        let err = store.save(&snapshot, 0).unwrap_err();
        assert!(matches!(err, Error::Validation(_)));
    }

    #[test]
    fn snapshot_rebuilds_from_lock_and_transition_events() {
        let tmp = TempDir::new().unwrap();
        let event_store = EventStore::new(tmp.path().join("events.jsonl"));
        let snapshot_store = SnapshotStore::new(
            tmp.path().join("tasks.snapshot.json"),
            tmp.path().join(".tasks.lock"),
        );

        let mut snapshot = TasksSnapshot::default();
        snapshot.upsert_task(sample_task("task-a"));
        snapshot_store.save(&snapshot, 0).unwrap();

        let base_ts = Utc::now();
        event_store
            .append(&TaskEvent {
                id: "1".into(),
                ts: base_ts,
                event_type: TaskEventType::StateTransition,
                task_id: "task-a".into(),
                agent: "codex".into(),
                node: "macbook".into(),
                payload: serde_json::json!({"to": "implementing"}),
                version: 1,
            })
            .unwrap();
        event_store
            .append(&TaskEvent {
                id: "2".into(),
                ts: base_ts + Duration::seconds(1),
                event_type: TaskEventType::LockAcquired,
                task_id: "task-a".into(),
                agent: "codex".into(),
                node: "macbook".into(),
                payload: serde_json::json!({
                    "files": ["src/lib.rs"],
                    "expires_at": base_ts + Duration::seconds(301),
                    "lease_duration_sec": 300
                }),
                version: 2,
            })
            .unwrap();

        let rebuilt = snapshot_store.rebuild(&event_store).unwrap();
        let task = rebuilt.get_task("task-a").unwrap();
        assert_eq!(task.current_state, TaskState::Implementing);
        assert!(task.lock.is_some());
        assert!(rebuilt.file_locks.contains_key("src/lib.rs"));
    }
}
