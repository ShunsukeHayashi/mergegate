//! Deterministic execution protocol entry point.

use crate::error::Error;
use crate::gate::{evaluate_gate, Gate, GateContext, GateReport};
use crate::lock::{FileLockManager, LeaseConfig, LockConflict};
use crate::store::{
    CompletionMode, EventStore, ExecutionTask, GitHubEvidence, GitHubIssueState, GitHubPrState,
    ImpactRiskLevel, ReviewDecision, SnapshotStore, TaskEvent, TaskEventType, TaskImpact,
    TaskState, TasksSnapshot,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
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

    pub fn from_store_path(store_path: impl Into<PathBuf>) -> Self {
        let store_path = store_path.into();
        let parent = store_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let event_store = EventStore::new(parent.join("task-events.jsonl"));
        let snapshot_store = SnapshotStore::new(store_path, parent.join(".tasks.lock"));
        let lock_manager = FileLockManager::new(
            event_store.clone(),
            snapshot_store.clone(),
            LeaseConfig::default(),
        );
        Self::new(event_store, snapshot_store, lock_manager)
    }

    pub fn run(
        &self,
        task_id: &str,
        gates: &[Gate],
        actor: &str,
        node: &str,
    ) -> ProtocolResult<ProtocolReport> {
        let start = Instant::now();
        let mut steps = Vec::new();
        let mut success = true;

        for gate in gates {
            let snapshot = self.snapshot_store.load().map_err(ProtocolError::from)?;
            let task = snapshot.get_task(task_id).ok_or_else(|| {
                ProtocolError::input(format!("unknown task: {task_id}"))
            })?;

            let context = if matches!(gate, Gate::Gate4) {
                let files = task
                    .lock
                    .as_ref()
                    .map(|lock| lock.affected_files.clone())
                    .unwrap_or_default();
                GateContext {
                    lock_conflict: Some(
                        self.lock_manager
                            .has_conflict(&files)
                            .map_err(ProtocolError::from)?,
                    ),
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

        let report = ProtocolReport {
            task_id: task_id.to_string(),
            steps,
            total_duration: start.elapsed(),
            success,
        };

        if report.success {
            Ok(report)
        } else {
            Err(ProtocolError::gate_rejected(
                report
                    .steps
                    .last()
                    .map(|step| step.detail.clone())
                    .unwrap_or_else(|| "gate rejected".to_string()),
            ))
        }
    }

    pub fn register(
        &self,
        request: RegisterTaskRequest,
        actor: &str,
        node: &str,
    ) -> ProtocolResult<ExecutionTask> {
        let mut snapshot = self.snapshot_store.load().map_err(ProtocolError::from)?;
        if snapshot.get_task(&request.task_id).is_some() {
            return Err(ProtocolError::input(format!(
                "task already exists: {}",
                request.task_id
            )));
        }

        let mut task = ExecutionTask::new(&request.task_id, request.title);
        task.current_state = TaskState::Pending;
        task.dependencies = request.dependencies;
        task.soft_dependencies = request.soft_dependencies;
        task.priority = request.priority;
        task.completion_mode = request.completion_mode;
        task.updated_at = Utc::now();
        snapshot.upsert_task(task.clone());
        recompute_dependents(&mut snapshot);
        let version = snapshot.version;
        self.snapshot_store
            .save(&snapshot, version)
            .map_err(ProtocolError::from)?;

        self.append_event(
            &task.id,
            TaskEventType::DagChanged,
            actor,
            node,
            version + 1,
            serde_json::json!({
                "title": task.title,
                "dependencies": task.dependencies,
                "soft_dependencies": task.soft_dependencies,
            }),
        )?;

        Ok(task)
    }

    pub fn status(&self, task_id: Option<&str>) -> ProtocolResult<StatusReport> {
        let snapshot = self.snapshot_store.load().map_err(ProtocolError::from)?;
        if let Some(task_id) = task_id {
            let task = snapshot
                .get_task(task_id)
                .cloned()
                .ok_or_else(|| ProtocolError::input(format!("unknown task: {task_id}")))?;
            Ok(StatusReport::Task(task))
        } else {
            Ok(StatusReport::Snapshot(snapshot))
        }
    }

    pub fn assign(
        &self,
        task_id: &str,
        agent: &str,
        node: &str,
        files: &[String],
    ) -> ProtocolResult<AssignmentResult> {
        self.run(task_id, &[Gate::Gate2], agent, node)?;
        let conflict = self
            .lock_manager
            .has_conflict(files)
            .map_err(ProtocolError::from)?;
        if conflict.conflicting {
            return Err(ProtocolError::gate_rejected(format!(
                "lock conflict held by {}",
                conflict.held_by.unwrap_or_else(|| "unknown".to_string())
            )));
        }

        self.lock_manager
            .acquire_lock(task_id, agent, node, files)
            .map_err(ProtocolError::from)?;

        let task = self.transition_task(task_id, TaskState::Implementing, agent, node)?;
        Ok(AssignmentResult {
            task,
            lock_conflict: LockConflict {
                conflicting: false,
                held_by: None,
                task_id: None,
            },
        })
    }

    pub fn record_impact(
        &self,
        task_id: &str,
        impact: ImpactInput,
        actor: &str,
        node: &str,
    ) -> ProtocolResult<ExecutionTask> {
        let payload_risk = impact.risk_level;
        let payload_symbols = impact.affected_symbols;
        let depth1 = impact.depth1.clone();
        let analyzed_commit = impact.analyzed_commit.clone();
        let input_hash = impact.input_hash.clone();
        self.update_task(task_id, actor, node, TaskEventType::ImpactRecorded, |task| {
            task.impact = Some(TaskImpact {
                risk_level: impact.risk_level,
                affected_symbols: impact.affected_symbols,
                depth1: depth1.clone(),
                analyzed_at: Utc::now(),
                analyzed_commit: analyzed_commit.clone(),
                input_hash: input_hash.clone(),
            });
            Ok(serde_json::json!({
                "risk_level": payload_risk,
                "affected_symbols": payload_symbols
            }))
        })
    }

    pub fn record_branch(
        &self,
        task_id: &str,
        branch_name: &str,
        actor: &str,
        node: &str,
    ) -> ProtocolResult<ExecutionTask> {
        if branch_name.trim().is_empty() {
            return Err(ProtocolError::input("branch name must not be empty"));
        }
        self.update_task(task_id, actor, node, TaskEventType::BranchCreated, |task| {
            task.branch_name = Some(branch_name.to_string());
            Ok(serde_json::json!({ "branch_name": branch_name }))
        })
    }

    pub fn record_pr(
        &self,
        task_id: &str,
        pr_number: u64,
        actor: &str,
        node: &str,
    ) -> ProtocolResult<ExecutionTask> {
        if pr_number == 0 {
            return Err(ProtocolError::input("pr number must be greater than 0"));
        }
        self.update_task(task_id, actor, node, TaskEventType::PrCreated, |task| {
            let branch_name = task.branch_name.clone().unwrap_or_default();
            task.github_evidence = Some(GitHubEvidence {
                pr_number,
                pr_head_ref: branch_name.clone(),
                pr_state: GitHubPrState::Open,
                merge_commit_sha: None,
                merged_at: None,
                review_decision: Some(ReviewDecision::ReviewRequired),
                issue_state: GitHubIssueState::Open,
                issue_closed_by_pr: false,
            });
            task.current_state = TaskState::Reviewing;
            Ok(serde_json::json!({ "pr_number": pr_number, "head_ref": branch_name }))
        })
    }

    pub fn record_merge(
        &self,
        task_id: &str,
        merge_commit_sha: &str,
        actor: &str,
        node: &str,
    ) -> ProtocolResult<ExecutionTask> {
        if merge_commit_sha.len() != 40 || !merge_commit_sha.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ProtocolError::input("merge sha must be a 40-char hex string"));
        }

        self.update_task(task_id, actor, node, TaskEventType::MergeVerified, |task| {
            let mut evidence = task.github_evidence.clone().ok_or_else(|| {
                ProtocolError::gate_rejected("pull request must be recorded before merge")
            })?;
            evidence.pr_state = GitHubPrState::Merged;
            evidence.merge_commit_sha = Some(merge_commit_sha.to_string());
            evidence.merged_at = Some(Utc::now());
            evidence.review_decision = Some(ReviewDecision::Approved);
            evidence.issue_state = GitHubIssueState::Closed;
            evidence.issue_closed_by_pr = true;
            task.github_evidence = Some(evidence);
            task.current_state = TaskState::Merged;
            Ok(serde_json::json!({ "merge_commit_sha": merge_commit_sha }))
        })
    }

    pub fn locks(&self) -> ProtocolResult<HashMap<String, crate::store::FileLockEntry>> {
        let snapshot = self.snapshot_store.load().map_err(ProtocolError::from)?;
        Ok(snapshot.file_locks)
    }

    pub fn dag(&self) -> ProtocolResult<DagReport> {
        let snapshot = self.snapshot_store.load().map_err(ProtocolError::from)?;
        Ok(compute_dag(&snapshot))
    }

    pub fn dispatchable(&self) -> ProtocolResult<DispatchableReport> {
        let snapshot = self.snapshot_store.load().map_err(ProtocolError::from)?;
        let tasks = snapshot
            .tasks
            .iter()
            .filter(|task| matches!(task.current_state, TaskState::Pending | TaskState::Blocked))
            .filter(|task| dependencies_satisfied(task, &snapshot))
            .filter(|task| {
                let files = task
                    .lock
                    .as_ref()
                    .map(|lock| lock.affected_files.clone())
                    .unwrap_or_default();
                !self
                    .lock_manager
                    .has_conflict(&files)
                    .map(|conflict| conflict.conflicting)
                    .unwrap_or(false)
            })
            .cloned()
            .collect();
        Ok(DispatchableReport { tasks })
    }

    fn transition_task(
        &self,
        task_id: &str,
        to: TaskState,
        actor: &str,
        node: &str,
    ) -> ProtocolResult<ExecutionTask> {
        self.update_task(task_id, actor, node, TaskEventType::StateTransition, |task| {
            task.current_state = to;
            Ok(serde_json::json!({ "to": to }))
        })
    }

    fn update_task<F>(
        &self,
        task_id: &str,
        actor: &str,
        node: &str,
        event_type: TaskEventType,
        mut update: F,
    ) -> ProtocolResult<ExecutionTask>
    where
        F: FnMut(&mut ExecutionTask) -> ProtocolResult<serde_json::Value>,
    {
        let mut snapshot = self.snapshot_store.load().map_err(ProtocolError::from)?;
        let task = snapshot
            .get_task_mut(task_id)
            .ok_or_else(|| ProtocolError::input(format!("unknown task: {task_id}")))?;
        let payload = update(task)?;
        task.updated_at = Utc::now();
        let updated = task.clone();
        let version = snapshot.version;
        self.snapshot_store
            .save(&snapshot, version)
            .map_err(ProtocolError::from)?;
        self.append_event(task_id, event_type, actor, node, version + 1, payload)?;
        Ok(updated)
    }

    fn append_event(
        &self,
        task_id: &str,
        event_type: TaskEventType,
        actor: &str,
        node: &str,
        version: u64,
        payload: serde_json::Value,
    ) -> ProtocolResult<()> {
        self.event_store
            .append(&TaskEvent {
                id: format!(
                    "{task_id}-{:?}-{}",
                    event_type,
                    Utc::now().timestamp_millis()
                ),
                ts: Utc::now(),
                event_type,
                task_id: task_id.to_string(),
                agent: actor.to_string(),
                node: node.to_string(),
                payload,
                version,
            })
            .map_err(ProtocolError::from)?;
        Ok(())
    }

    fn record_gate(
        &self,
        task_id: &str,
        gate: Gate,
        report: &GateReport,
        actor: &str,
        node: &str,
        version: u64,
    ) -> ProtocolResult<()> {
        let event_type = if report.success {
            TaskEventType::GatePassed
        } else {
            TaskEventType::GateRejected
        };
        self.append_event(
            task_id,
            event_type,
            actor,
            node,
            version,
            serde_json::json!({
                "gate": gate.label(),
                "detail": report.detail,
                "duration_ms": report.duration.as_millis(),
            }),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolReport {
    pub task_id: String,
    pub steps: Vec<GateReport>,
    pub total_duration: Duration,
    pub success: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisterTaskRequest {
    pub task_id: String,
    pub title: String,
    pub dependencies: Vec<String>,
    pub soft_dependencies: Vec<String>,
    pub priority: u32,
    pub completion_mode: CompletionMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssignmentResult {
    pub task: ExecutionTask,
    pub lock_conflict: LockConflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImpactInput {
    pub risk_level: ImpactRiskLevel,
    pub affected_symbols: usize,
    pub depth1: Vec<String>,
    pub analyzed_commit: Option<String>,
    pub input_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StatusReport {
    Task(ExecutionTask),
    Snapshot(TasksSnapshot),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DagReport {
    pub levels: Vec<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchableReport {
    pub tasks: Vec<ExecutionTask>,
}

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("gate rejected: {0}")]
    GateRejected(String),
    #[error("input error: {0}")]
    Input(String),
    #[error("{0}")]
    Internal(#[from] Error),
}

impl ProtocolError {
    pub fn gate_rejected(msg: impl Into<String>) -> Self {
        Self::GateRejected(msg.into())
    }

    pub fn input(msg: impl Into<String>) -> Self {
        Self::Input(msg.into())
    }
}

pub type ProtocolResult<T> = std::result::Result<T, ProtocolError>;

fn dependencies_satisfied(task: &ExecutionTask, snapshot: &TasksSnapshot) -> bool {
    task.dependencies.iter().all(|dep_id| {
        snapshot.get_task(dep_id).is_some_and(|dep| {
            matches!(dep.current_state, TaskState::Done | TaskState::Merged)
        })
    })
}

fn recompute_dependents(snapshot: &mut TasksSnapshot) {
    let mut dependents: HashMap<String, Vec<String>> = HashMap::new();
    for task in &snapshot.tasks {
        for dependency in &task.dependencies {
            dependents
                .entry(dependency.clone())
                .or_default()
                .push(task.id.clone());
        }
    }

    for task in &mut snapshot.tasks {
        task.dependents = dependents.remove(&task.id).unwrap_or_default();
    }
}

fn compute_dag(snapshot: &TasksSnapshot) -> DagReport {
    let mut indegree: HashMap<String, usize> = snapshot
        .tasks
        .iter()
        .map(|task| (task.id.clone(), task.dependencies.len()))
        .collect();
    let mut queue: VecDeque<String> = snapshot
        .tasks
        .iter()
        .filter(|task| task.dependencies.is_empty())
        .map(|task| task.id.clone())
        .collect();
    let mut levels = Vec::new();
    let task_map: HashMap<&str, &ExecutionTask> =
        snapshot.tasks.iter().map(|task| (task.id.as_str(), task)).collect();

    while !queue.is_empty() {
        let mut next = VecDeque::new();
        let mut level = Vec::new();

        while let Some(task_id) = queue.pop_front() {
            level.push(task_id.clone());
            for dependent in snapshot
                .tasks
                .iter()
                .filter(|task| task.dependencies.contains(&task_id))
            {
                if let Some(entry) = indegree.get_mut(&dependent.id) {
                    *entry = entry.saturating_sub(1);
                    if *entry == 0 {
                        next.push_back(dependent.id.clone());
                    }
                }
            }
        }

        levels.push(level);
        queue = next;
    }

    for task in &snapshot.tasks {
        if task_map.contains_key(task.id.as_str())
            && !levels.iter().flatten().any(|task_id| task_id == &task.id)
        {
            levels.push(vec![task.id.clone()]);
        }
    }

    DagReport { levels }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fixture() -> (TempDir, DeterministicExecutionProtocol) {
        let tmp = TempDir::new().unwrap();
        let protocol =
            DeterministicExecutionProtocol::from_store_path(tmp.path().join("tasks.json"));
        (tmp, protocol)
    }

    #[test]
    fn protocol_stops_at_first_failed_gate_and_records_events() {
        let (_tmp, protocol) = fixture();
        protocol
            .register(
                RegisterTaskRequest {
                    task_id: "phase-0".into(),
                    title: "Phase 0".into(),
                    dependencies: vec![],
                    soft_dependencies: vec![],
                    priority: 0,
                    completion_mode: CompletionMode::GithubPr,
                },
                "codex",
                "macbook",
            )
            .unwrap();
        protocol
            .register(
                RegisterTaskRequest {
                    task_id: "phase-a".into(),
                    title: "Phase A".into(),
                    dependencies: vec!["phase-0".into()],
                    soft_dependencies: vec![],
                    priority: 0,
                    completion_mode: CompletionMode::GithubPr,
                },
                "codex",
                "macbook",
            )
            .unwrap();

        let err = protocol
            .run(
                "phase-a",
                &[Gate::Gate1, Gate::Gate2, Gate::Gate3],
                "codex",
                "macbook",
            )
            .unwrap_err();
        assert!(matches!(err, ProtocolError::GateRejected(_)));

        let events = protocol.event_store.replay_for_task("phase-a").unwrap();
        assert_eq!(events.len(), 3);
        assert_eq!(events[1].event_type, TaskEventType::GatePassed);
        assert_eq!(events[2].event_type, TaskEventType::GateRejected);
    }

    #[test]
    fn register_status_and_dispatchable_flow() {
        let (_tmp, protocol) = fixture();
        protocol
            .register(
                RegisterTaskRequest {
                    task_id: "phase-a".into(),
                    title: "Phase A".into(),
                    dependencies: vec![],
                    soft_dependencies: vec![],
                    priority: 10,
                    completion_mode: CompletionMode::GithubPr,
                },
                "codex",
                "macbook",
            )
            .unwrap();

        let status = protocol.status(Some("phase-a")).unwrap();
        match status {
            StatusReport::Task(task) => assert_eq!(task.title, "Phase A"),
            StatusReport::Snapshot(_) => panic!("expected task status"),
        }

        let dispatchable = protocol.dispatchable().unwrap();
        assert_eq!(dispatchable.tasks.len(), 1);
        assert_eq!(dispatchable.tasks[0].id, "phase-a");
    }

    #[test]
    fn assign_branch_pr_merge_updates_task() {
        let (_tmp, protocol) = fixture();
        protocol
            .register(
                RegisterTaskRequest {
                    task_id: "phase-a".into(),
                    title: "Phase A".into(),
                    dependencies: vec![],
                    soft_dependencies: vec![],
                    priority: 0,
                    completion_mode: CompletionMode::GithubPr,
                },
                "codex",
                "macbook",
            )
            .unwrap();
        protocol
            .record_impact(
                "phase-a",
                ImpactInput {
                    risk_level: ImpactRiskLevel::Low,
                    affected_symbols: 1,
                    depth1: vec!["main".into()],
                    analyzed_commit: None,
                    input_hash: None,
                },
                "codex",
                "macbook",
            )
            .unwrap();

        let assigned = protocol
            .assign(
                "phase-a",
                "codex",
                "macbook",
                &[String::from("crates/miyabi-core/src/main.rs")],
            )
            .unwrap();
        assert_eq!(assigned.task.current_state, TaskState::Implementing);

        protocol
            .record_branch("phase-a", "feature/phase-a", "codex", "macbook")
            .unwrap();
        protocol.record_pr("phase-a", 12, "codex", "macbook").unwrap();
        let merged = protocol
            .record_merge(
                "phase-a",
                "0123456789abcdef0123456789abcdef01234567",
                "codex",
                "macbook",
            )
            .unwrap();

        assert_eq!(merged.current_state, TaskState::Merged);
        assert_eq!(
            merged
                .github_evidence
                .as_ref()
                .unwrap()
                .merge_commit_sha
                .as_deref(),
            Some("0123456789abcdef0123456789abcdef01234567")
        );
    }
}
