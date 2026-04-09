//! Deterministic execution protocol entry point.

use crate::error::{Error, Result};
use crate::gate::{evaluate_gate, Gate, GateContext, GateReport};
use crate::lock::FileLockManager;
use crate::store::{EventStore, SnapshotStore, TaskEvent, TaskEventType};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct DeterministicExecutionProtocol {
    event_store: EventStore,
    snapshot_store: SnapshotStore,
    lock_manager: FileLockManager,
}

impl DeterministicExecutionProtocol {
    pub fn new(
        event_store: EventStore,
        snapshot_store: SnapshotStore,
        lock_manager: FileLockManager,
    ) -> Self {
        Self {
            event_store,
            snapshot_store,
            lock_manager,
        }
    }

    pub fn run(
        &self,
        task_id: &str,
        gates: &[Gate],
        actor: &str,
        node: &str,
    ) -> Result<ProtocolReport> {
        let start = Instant::now();
        let mut steps = Vec::new();
        let mut success = true;

        for gate in gates {
            let snapshot = self.snapshot_store.load()?;
            let task = snapshot
                .get_task(task_id)
                .ok_or_else(|| Error::Validation(format!("unknown task: {task_id}")))?;

            let context = if matches!(gate, Gate::Gate4) {
                let files = task
                    .lock
                    .as_ref()
                    .map(|lock| lock.affected_files.clone())
                    .unwrap_or_default();
                GateContext {
                    lock_conflict: Some(self.lock_manager.has_conflict(&files)?),
                }
            } else {
                GateContext::default()
            };

            let report = evaluate_gate(*gate, task, &snapshot, &context);
            self.record_gate(task_id, *gate, &report, actor, node, snapshot.version + 1)?;
            success &= report.success;
            steps.push(report);

            if !success {
                break;
            }
        }

        Ok(ProtocolReport {
            task_id: task_id.to_string(),
            steps,
            total_duration: start.elapsed(),
            success,
        })
    }

    fn record_gate(
        &self,
        task_id: &str,
        gate: Gate,
        report: &GateReport,
        actor: &str,
        node: &str,
        version: u64,
    ) -> Result<()> {
        let event_type = if report.success {
            TaskEventType::GatePassed
        } else {
            TaskEventType::GateRejected
        };
        self.event_store.append(&TaskEvent {
            id: format!(
                "{task_id}-{}-{}",
                gate.label(),
                Utc::now().timestamp_millis()
            ),
            ts: Utc::now(),
            event_type,
            task_id: task_id.to_string(),
            agent: actor.to_string(),
            node: node.to_string(),
            payload: serde_json::json!({
                "gate": gate.label(),
                "detail": report.detail,
                "duration_ms": report.duration.as_millis(),
            }),
            version,
        })?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolReport {
    pub task_id: String,
    pub steps: Vec<GateReport>,
    pub total_duration: Duration,
    pub success: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lock::LeaseConfig;
    use crate::store::{ExecutionTask, SnapshotStore, TasksSnapshot};
    use tempfile::TempDir;

    #[test]
    fn protocol_stops_at_first_failed_gate_and_records_events() {
        let tmp = TempDir::new().unwrap();
        let event_store = EventStore::new(tmp.path().join("events.jsonl"));
        let snapshot_store = SnapshotStore::new(
            tmp.path().join("tasks.snapshot.json"),
            tmp.path().join(".tasks.lock"),
        );
        let lock_manager = FileLockManager::new(
            event_store.clone(),
            snapshot_store.clone(),
            LeaseConfig::default(),
        );
        let protocol = DeterministicExecutionProtocol::new(
            event_store.clone(),
            snapshot_store.clone(),
            lock_manager,
        );

        let mut snapshot = TasksSnapshot::default();
        let mut task = ExecutionTask::new("phase-a", "Phase A");
        task.current_state = crate::store::TaskState::Pending;
        task.dependencies.push("phase-0".into());
        snapshot.upsert_task(task);
        snapshot.upsert_task(ExecutionTask::new("phase-0", "Phase 0"));
        snapshot_store.save(&snapshot, 0).unwrap();

        let report = protocol
            .run(
                "phase-a",
                &[Gate::Gate1, Gate::Gate2, Gate::Gate3],
                "codex",
                "macbook",
            )
            .unwrap();

        assert!(!report.success);
        assert_eq!(report.steps.len(), 2);
        assert!(report.steps[0].success);
        assert!(!report.steps[1].success);

        let events = event_store.replay_for_task("phase-a").unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, TaskEventType::GatePassed);
        assert_eq!(events[1].event_type, TaskEventType::GateRejected);
    }
}
