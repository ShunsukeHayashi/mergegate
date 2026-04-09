//! Deterministic execution protocol entry point.

use crate::dream::DreamReport;
use crate::error::Error;
use crate::gate::{evaluate_gate, validate_branch_name, Gate, GateContext, GateReport};
use crate::lock::{FileLockManager, LeaseConfig, LockConflict};
use crate::store::{
    CompletionMode, ContextAttachment, EventStore, ExecutionTask, GitHubEvidence, GitHubIssueState,
    GitHubPrState, HumanApproval, ImpactRiskLevel, ReviewDecision, SnapshotStore, TaskEvent,
    TaskEventType, TaskImpact, TaskState, TasksSnapshot,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const MAX_CONTEXT_TOKENS: usize = 4_000;
const FILE_SNIPPET_LINE_LIMIT: usize = 30;

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
            let task = snapshot
                .get_task(task_id)
                .ok_or_else(|| ProtocolError::input(format!("unknown task: {task_id}")))?;

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
        if request.issue == 0 {
            return Err(ProtocolError::input("issue number must be greater than 0"));
        }
        let mut snapshot = self.snapshot_store.load().map_err(ProtocolError::from)?;
        if snapshot.get_task(&request.task_id).is_some() {
            return Err(ProtocolError::input(format!(
                "task already exists: {}",
                request.task_id
            )));
        }

        let mut task = ExecutionTask::new(&request.task_id, request.title);
        task.issue_number = request.issue;
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
            Ok(StatusReport::Task(Box::new(task)))
        } else {
            Ok(StatusReport::Snapshot(Box::new(snapshot)))
        }
    }

    pub fn assign(
        &self,
        task_id: &str,
        agent: &str,
        node: &str,
        files: &[String],
    ) -> ProtocolResult<AssignmentResult> {
        let snapshot = self.snapshot_store.load().map_err(ProtocolError::from)?;
        let task = snapshot
            .get_task(task_id)
            .ok_or_else(|| ProtocolError::input(format!("unknown task: {task_id}")))?;
        let approval_granted = task
            .human_approval
            .as_ref()
            .is_some_and(|approval| !approval.required || approval.approved_by.is_some());
        if matches!(
            task.impact.as_ref().map(|impact| impact.risk_level),
            Some(ImpactRiskLevel::High | ImpactRiskLevel::Critical)
        ) && !approval_granted
        {
            return Err(ProtocolError::gate_rejected(
                "HIGH/CRITICAL risk requires --approve",
            ));
        }

        let blocked_by = task
            .dependencies
            .iter()
            .filter_map(|dependency| {
                snapshot
                    .get_task(dependency)
                    .filter(|dep| !matches!(dep.current_state, TaskState::Done | TaskState::Merged))
                    .map(|_| dependency.clone())
            })
            .collect::<Vec<_>>();
        if !blocked_by.is_empty() {
            return Err(ProtocolError::dependency_blocked(format!(
                "blocked by dependencies: {}",
                blocked_by.join(", ")
            )));
        }

        self.run(task_id, &[Gate::Gate2], agent, node)?;
        let conflict = self
            .lock_manager
            .has_conflict(files)
            .map_err(ProtocolError::from)?;
        if conflict.conflicting {
            self.append_event(
                task_id,
                TaskEventType::GateRejected,
                agent,
                node,
                snapshot.version + 1,
                serde_json::json!({
                    "gate": Gate::Gate4.label(),
                    "detail": format!(
                        "lock conflict held by {}",
                        conflict.held_by.as_deref().unwrap_or("unknown")
                    ),
                    "files": files,
                }),
            )?;
            return Err(ProtocolError::gate_rejected(format!(
                "lock conflict held by {}",
                conflict.held_by.unwrap_or_else(|| "unknown".to_string())
            )));
        }

        self.lock_manager
            .acquire_lock(task_id, agent, node, files)
            .map_err(ProtocolError::from)?;
        self.attach_context(task_id, agent, node)?;

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
        let ImpactInput {
            risk_level,
            affected_symbols,
            depth1,
            analyzed_commit,
            input_hash,
            approve,
        } = impact;
        let approval = approve.then(|| HumanApproval {
            required: matches!(
                risk_level,
                ImpactRiskLevel::High | ImpactRiskLevel::Critical
            ),
            approved_by: Some(actor.to_string()),
            approved_at: Some(Utc::now()),
            reason: Some("approved via impact --approve".to_string()),
        });
        self.update_task(
            task_id,
            actor,
            node,
            TaskEventType::ImpactRecorded,
            |task| {
                task.impact = Some(TaskImpact {
                    risk_level,
                    affected_symbols,
                    depth1: depth1.clone(),
                    analyzed_at: Utc::now(),
                    analyzed_commit: analyzed_commit.clone(),
                    input_hash: input_hash.clone(),
                });
                task.human_approval = approval.clone();
                Ok(serde_json::json!({
                    "risk_level": risk_level,
                    "affected_symbols": affected_symbols,
                    "approve": approve,
                    "human_approval": approval.clone()
                }))
            },
        )
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
        if !validate_branch_name(branch_name) {
            return Err(ProtocolError::gate_rejected(format!(
                "invalid branch name: {branch_name}"
            )));
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
        if merge_commit_sha.len() != 40 || !merge_commit_sha.chars().all(|c| c.is_ascii_hexdigit())
        {
            return Err(ProtocolError::gate_rejected(
                "merge sha must be a 40-char hex string",
            ));
        }

        let merged =
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
            })?;

        self.lock_manager
            .release_lock(task_id)
            .map_err(ProtocolError::from)?;
        self.unblock_dependents_after_merge(task_id, actor, node)?;

        let snapshot = self.snapshot_store.load().map_err(ProtocolError::from)?;
        Ok(snapshot.get_task(task_id).cloned().unwrap_or(merged))
    }

    pub fn locks(&self) -> ProtocolResult<HashMap<String, crate::store::FileLockEntry>> {
        let snapshot = self.snapshot_store.load().map_err(ProtocolError::from)?;
        Ok(snapshot.file_locks)
    }

    pub fn dag(&self) -> ProtocolResult<DagReport> {
        let snapshot = self.snapshot_store.load().map_err(ProtocolError::from)?;
        Ok(compute_dag(&snapshot))
    }

    pub fn dream(
        &self,
        since: Option<chrono::Duration>,
        auto: bool,
        repo_root: &Path,
        actor: &str,
        node: &str,
    ) -> ProtocolResult<DreamReport> {
        let report = crate::dream::dream_with_auto(&self.event_store, since, auto, repo_root)
            .map_err(ProtocolError::from)?;
        let version = self
            .snapshot_store
            .load()
            .map_err(ProtocolError::from)?
            .version;
        self.append_event(
            "__dream__",
            TaskEventType::DreamRecorded,
            actor,
            node,
            version,
            serde_json::json!({
                "events_processed": report.events_processed,
                "gate_rejections": report.patterns.gate_rejections.len(),
                "lock_conflicts": report.patterns.lock_conflicts.len(),
                "learnings": report.learnings.len(),
            }),
        )?;
        Ok(report)
    }

    pub fn attach_context(
        &self,
        task_id: &str,
        actor: &str,
        node: &str,
    ) -> ProtocolResult<Vec<ContextAttachment>> {
        self.attach_context_with_limit(task_id, actor, node, MAX_CONTEXT_TOKENS)
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

    fn attach_context_with_limit(
        &self,
        task_id: &str,
        actor: &str,
        node: &str,
        max_context_tokens: usize,
    ) -> ProtocolResult<Vec<ContextAttachment>> {
        let snapshot = self.snapshot_store.load().map_err(ProtocolError::from)?;
        let task = snapshot
            .get_task(task_id)
            .cloned()
            .ok_or_else(|| ProtocolError::input(format!("unknown task: {task_id}")))?;
        let attachments = self.build_context_attachments(&task, max_context_tokens)?;
        let payload = serde_json::to_value(&attachments).map_err(Error::from)?;
        self.update_task(
            task_id,
            actor,
            node,
            TaskEventType::ContextAttached,
            |task| {
                task.context_attachments = attachments.clone();
                Ok(payload.clone())
            },
        )?;
        Ok(attachments)
    }

    fn transition_task(
        &self,
        task_id: &str,
        to: TaskState,
        actor: &str,
        node: &str,
    ) -> ProtocolResult<ExecutionTask> {
        self.update_task(
            task_id,
            actor,
            node,
            TaskEventType::StateTransition,
            |task| {
                task.current_state = to;
                Ok(serde_json::json!({ "to": to }))
            },
        )
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

    fn build_context_attachments(
        &self,
        task: &ExecutionTask,
        max_context_tokens: usize,
    ) -> ProtocolResult<Vec<ContextAttachment>> {
        let mut attachments = Vec::new();
        let mut remaining_tokens = max_context_tokens;

        if task.issue_number > 0 {
            push_attachment(
                &mut attachments,
                &mut remaining_tokens,
                "issue",
                &format!("github://issue/{}", task.issue_number),
                &format!("Issue #{}", task.issue_number),
            );
        }

        if let Some(impact) = &task.impact {
            push_attachment(
                &mut attachments,
                &mut remaining_tokens,
                "impact",
                &format!("dtp://impact/{}", task.id),
                &format!(
                    "risk_level: {:?}\naffected_symbols: {}",
                    impact.risk_level, impact.affected_symbols
                ),
            );
        }

        if let Some(vault_path) = obsidian_vault_path() {
            for note in find_obsidian_notes(&vault_path, &task.title, 3)? {
                if remaining_tokens == 0 {
                    break;
                }
                let content = read_file_snippet(&note, FILE_SNIPPET_LINE_LIMIT)
                    .map_err(ProtocolError::from)?;
                push_attachment(
                    &mut attachments,
                    &mut remaining_tokens,
                    "obsidian_note",
                    &note.display().to_string(),
                    &content,
                );
            }
        }

        if let Some(lock) = &task.lock {
            for file in &lock.affected_files {
                if remaining_tokens == 0 {
                    break;
                }
                let source_path = self.resolve_attachment_path(file);
                if !source_path.exists() {
                    continue;
                }
                let content = read_file_snippet(&source_path, FILE_SNIPPET_LINE_LIMIT)
                    .map_err(ProtocolError::from)?;
                push_attachment(
                    &mut attachments,
                    &mut remaining_tokens,
                    "file_snippet",
                    &source_path.display().to_string(),
                    &content,
                );
            }
        }

        Ok(attachments)
    }

    fn resolve_attachment_path(&self, source: &str) -> PathBuf {
        let path = PathBuf::from(source);
        if path.is_absolute() {
            return path;
        }

        let store_relative = self
            .snapshot_store
            .path()
            .parent()
            .map(|base| base.join(path))
            .unwrap_or_else(|| PathBuf::from(source));
        if store_relative.exists() {
            return store_relative;
        }

        let cwd_relative = std::env::current_dir()
            .map(|cwd| cwd.join(source))
            .unwrap_or_else(|_| PathBuf::from(source));
        if cwd_relative.exists() {
            return cwd_relative;
        }

        store_relative
    }

    fn unblock_dependents_after_merge(
        &self,
        task_id: &str,
        actor: &str,
        node: &str,
    ) -> ProtocolResult<()> {
        let mut snapshot = self.snapshot_store.load().map_err(ProtocolError::from)?;
        let dependent_ids = snapshot
            .get_task(task_id)
            .ok_or_else(|| ProtocolError::input(format!("unknown task: {task_id}")))?
            .dependents
            .clone();

        let pending_dependents: Vec<String> = dependent_ids
            .into_iter()
            .filter(|dependent_id| {
                snapshot.get_task(dependent_id).is_some_and(|task| {
                    task.current_state == TaskState::Blocked
                        && dependencies_satisfied(task, &snapshot)
                })
            })
            .collect();

        if pending_dependents.is_empty() {
            return Ok(());
        }

        let now = Utc::now();
        for dependent_id in &pending_dependents {
            if let Some(task) = snapshot.get_task_mut(dependent_id) {
                task.current_state = TaskState::Pending;
                task.updated_at = now;
            }
        }

        let version = snapshot.version;
        self.snapshot_store
            .save(&snapshot, version)
            .map_err(ProtocolError::from)?;
        for dependent_id in pending_dependents {
            self.append_event(
                &dependent_id,
                TaskEventType::StateTransition,
                actor,
                node,
                version + 1,
                serde_json::json!({
                    "to": TaskState::Pending,
                    "reason": "dependencies_resolved_after_merge",
                    "unblocked_by": task_id,
                }),
            )?;
        }

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisterTaskRequest {
    pub issue: u64,
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
    pub approve: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StatusReport {
    Task(Box<ExecutionTask>),
    Snapshot(Box<TasksSnapshot>),
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
    #[error("dependency blocked: {0}")]
    DependencyBlocked(String),
    #[error("input error: {0}")]
    Input(String),
    #[error("{0}")]
    Internal(#[from] Error),
}

impl ProtocolError {
    pub fn gate_rejected(msg: impl Into<String>) -> Self {
        Self::GateRejected(msg.into())
    }

    pub fn dependency_blocked(msg: impl Into<String>) -> Self {
        Self::DependencyBlocked(msg.into())
    }

    pub fn input(msg: impl Into<String>) -> Self {
        Self::Input(msg.into())
    }
}

pub type ProtocolResult<T> = std::result::Result<T, ProtocolError>;

fn dependencies_satisfied(task: &ExecutionTask, snapshot: &TasksSnapshot) -> bool {
    task.dependencies.iter().all(|dep_id| {
        snapshot
            .get_task(dep_id)
            .is_some_and(|dep| matches!(dep.current_state, TaskState::Done | TaskState::Merged))
    })
}

fn push_attachment(
    attachments: &mut Vec<ContextAttachment>,
    remaining_tokens: &mut usize,
    attachment_type: &str,
    source: &str,
    content: &str,
) {
    if *remaining_tokens == 0 {
        return;
    }

    let truncated_content = truncate_to_token_budget(content, *remaining_tokens);
    let token_estimate = estimate_tokens(&truncated_content);
    if token_estimate == 0 {
        return;
    }

    *remaining_tokens = remaining_tokens.saturating_sub(token_estimate);
    attachments.push(ContextAttachment {
        attachment_type: attachment_type.to_string(),
        source: source.to_string(),
        content: truncated_content,
        token_estimate,
    });
}

fn truncate_to_token_budget(content: &str, max_tokens: usize) -> String {
    if max_tokens == 0 {
        return String::new();
    }

    let max_chars = max_tokens.saturating_mul(4);
    let content_chars = content.chars().count();
    if content_chars <= max_chars {
        return content.to_string();
    }

    let truncated: String = content.chars().take(max_chars).collect();
    if max_chars >= 3 {
        format!(
            "{}...",
            truncated.chars().take(max_chars - 3).collect::<String>()
        )
    } else {
        truncated
    }
}

fn estimate_tokens(content: &str) -> usize {
    let char_count = content.chars().count();
    if char_count == 0 {
        0
    } else {
        char_count.div_ceil(4)
    }
}

fn read_file_snippet(path: &Path, max_lines: usize) -> Result<String, Error> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines = Vec::new();
    for line in reader.lines().take(max_lines) {
        lines.push(line?);
    }
    Ok(lines.join("\n"))
}

fn obsidian_vault_path() -> Option<PathBuf> {
    std::env::var("OBSIDIAN_VAULT_PATH")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn find_obsidian_notes(
    vault_path: &Path,
    task_title: &str,
    limit: usize,
) -> Result<Vec<PathBuf>, Error> {
    if limit == 0 || !vault_path.exists() {
        return Ok(Vec::new());
    }

    let keywords = title_keywords(task_title);
    if keywords.is_empty() {
        return Ok(Vec::new());
    }

    let mut matches = Vec::new();
    collect_obsidian_matches(vault_path, &keywords, &mut matches)?;
    matches.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.as_os_str().cmp(right.1.as_os_str()))
    });
    matches.truncate(limit);
    Ok(matches.into_iter().map(|(_, path)| path).collect())
}

fn collect_obsidian_matches(
    directory: &Path,
    keywords: &[String],
    matches: &mut Vec<(usize, PathBuf)>,
) -> Result<(), Error> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_obsidian_matches(&path, keywords, matches)?;
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }

        let haystack = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let score = keywords
            .iter()
            .filter(|keyword| haystack.contains(keyword.as_str()))
            .count();
        if score > 0 {
            matches.push((score, path));
        }
    }

    Ok(())
}

fn title_keywords(title: &str) -> Vec<String> {
    let normalized: String = title
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect();
    normalized
        .split_whitespace()
        .filter(|token| token.len() >= 3)
        .map(ToString::to_string)
        .collect()
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
    let task_map: HashMap<&str, &ExecutionTask> = snapshot
        .tasks
        .iter()
        .map(|task| (task.id.as_str(), task))
        .collect();

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
    use std::fs;
    use std::sync::{Mutex, OnceLock};
    use tempfile::TempDir;

    fn obsidian_env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

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
                    issue: 1,
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
                    issue: 2,
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
                    issue: 1,
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
    fn register_rejects_issue_zero() {
        let (_tmp, protocol) = fixture();
        let err = protocol
            .register(
                RegisterTaskRequest {
                    issue: 0,
                    task_id: "test-task".into(),
                    title: "Test Task".into(),
                    dependencies: vec![],
                    soft_dependencies: vec![],
                    priority: 0,
                    completion_mode: CompletionMode::GithubPr,
                },
                "codex",
                "macbook",
            )
            .unwrap_err();

        assert!(matches!(err, ProtocolError::Input(_)));
        assert!(err
            .to_string()
            .contains("issue number must be greater than 0"));
    }

    #[test]
    fn assign_branch_pr_merge_updates_task() {
        let (_tmp, protocol) = fixture();
        protocol
            .register(
                RegisterTaskRequest {
                    issue: 1,
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
                    approve: false,
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
            .record_branch("phase-a", "feature/issue-1-phase-a", "codex", "macbook")
            .unwrap();
        protocol
            .record_pr("phase-a", 12, "codex", "macbook")
            .unwrap();
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
        assert!(protocol.locks().unwrap().is_empty());
    }

    #[test]
    fn attach_context_collects_issue_impact_and_file_snippets() {
        let _guard = obsidian_env_lock().lock().unwrap();
        std::env::remove_var("OBSIDIAN_VAULT_PATH");
        let (tmp, protocol) = fixture();
        let src_dir = tmp.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        let file_path = src_dir.join("lib.rs");
        let file_content = (1..=35)
            .map(|line| format!("line {line}"))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(&file_path, file_content).unwrap();

        protocol
            .register(
                RegisterTaskRequest {
                    issue: 42,
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
                    affected_symbols: 7,
                    depth1: vec!["attach_context".into()],
                    analyzed_commit: None,
                    input_hash: None,
                    approve: false,
                },
                "codex",
                "macbook",
            )
            .unwrap();
        protocol
            .assign("phase-a", "codex", "macbook", &[String::from("src/lib.rs")])
            .unwrap();

        let attachments = protocol
            .attach_context("phase-a", "codex", "macbook")
            .unwrap();

        let issue_attachment = attachments
            .iter()
            .find(|attachment| attachment.attachment_type == "issue")
            .unwrap();
        assert_eq!(issue_attachment.content, "Issue #42");
        let impact_attachment = attachments
            .iter()
            .find(|attachment| attachment.attachment_type == "impact")
            .unwrap();
        assert!(impact_attachment.content.contains("affected_symbols: 7"));
        let file_attachment = attachments
            .iter()
            .find(|attachment| attachment.attachment_type == "file_snippet")
            .unwrap();
        assert!(file_attachment.content.contains("line 1"));
        assert!(file_attachment.content.contains("line 30"));
        assert!(!file_attachment.content.contains("line 31"));

        let task = match protocol.status(Some("phase-a")).unwrap() {
            StatusReport::Task(task) => task,
            StatusReport::Snapshot(_) => panic!("expected task status"),
        };
        assert_eq!(task.issue_number, 42);
        assert!(task.context_attachments.len() >= 3);
    }

    #[test]
    fn attach_context_trims_to_token_budget() {
        let _guard = obsidian_env_lock().lock().unwrap();
        std::env::remove_var("OBSIDIAN_VAULT_PATH");
        let (tmp, protocol) = fixture();
        let src_dir = tmp.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(
            src_dir.join("big.rs"),
            "abcdefghijklmnopqrstuvwxyz".repeat(40),
        )
        .unwrap();

        protocol
            .register(
                RegisterTaskRequest {
                    issue: 9,
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
                    affected_symbols: 99,
                    depth1: vec!["attach_context".into()],
                    analyzed_commit: None,
                    input_hash: None,
                    approve: false,
                },
                "codex",
                "macbook",
            )
            .unwrap();
        protocol
            .assign("phase-a", "codex", "macbook", &[String::from("src/big.rs")])
            .unwrap();

        let attachments = protocol
            .attach_context_with_limit("phase-a", "codex", "macbook", 8)
            .unwrap();

        let total_tokens: usize = attachments.iter().map(|item| item.token_estimate).sum();
        assert!(total_tokens <= 8);
        assert!(!attachments.is_empty());
    }

    #[test]
    fn attach_context_with_obsidian_vault_finds_matching_notes() {
        let _guard = obsidian_env_lock().lock().unwrap();
        let (tmp, protocol) = fixture();
        let src_dir = tmp.path().join("src");
        let vault_dir = tmp.path().join("vault");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(vault_dir.join("Notes")).unwrap();
        fs::write(src_dir.join("lib.rs"), "fn main() {}\n").unwrap();
        fs::write(
            vault_dir.join("Notes").join("phase-auth-implementation.md"),
            "# Auth notes\n",
        )
        .unwrap();
        fs::write(
            vault_dir.join("Notes").join("unrelated-topic.md"),
            "# Other\n",
        )
        .unwrap();
        std::env::set_var("OBSIDIAN_VAULT_PATH", &vault_dir);

        protocol
            .register(
                RegisterTaskRequest {
                    issue: 10,
                    task_id: "phase-a".into(),
                    title: "Phase Auth Implementation".into(),
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
                    depth1: vec!["attach_context".into()],
                    analyzed_commit: None,
                    input_hash: None,
                    approve: false,
                },
                "codex",
                "macbook",
            )
            .unwrap();
        protocol
            .assign("phase-a", "codex", "macbook", &[String::from("src/lib.rs")])
            .unwrap();

        let attachments = protocol
            .attach_context("phase-a", "codex", "macbook")
            .unwrap();
        std::env::remove_var("OBSIDIAN_VAULT_PATH");

        assert!(attachments.iter().any(|attachment| {
            attachment.attachment_type == "obsidian_note"
                && attachment.source.ends_with("phase-auth-implementation.md")
        }));
    }

    #[test]
    fn assign_rejects_high_risk_without_human_approval() {
        let (_tmp, protocol) = fixture();
        protocol
            .register(
                RegisterTaskRequest {
                    issue: 1,
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
                    risk_level: ImpactRiskLevel::High,
                    affected_symbols: 2,
                    depth1: vec!["assign".into()],
                    analyzed_commit: None,
                    input_hash: None,
                    approve: false,
                },
                "codex",
                "macbook",
            )
            .unwrap();

        let err = protocol
            .assign("phase-a", "codex", "macbook", &[String::from("src/lib.rs")])
            .unwrap_err();

        assert!(matches!(err, ProtocolError::GateRejected(_)));
        assert_eq!(
            err.to_string(),
            "gate rejected: HIGH/CRITICAL risk requires --approve"
        );
    }

    #[test]
    fn impact_approve_records_human_approval() {
        let (_tmp, protocol) = fixture();
        protocol
            .register(
                RegisterTaskRequest {
                    issue: 1,
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

        let task = protocol
            .record_impact(
                "phase-a",
                ImpactInput {
                    risk_level: ImpactRiskLevel::High,
                    affected_symbols: 2,
                    depth1: vec!["assign".into()],
                    analyzed_commit: None,
                    input_hash: None,
                    approve: true,
                },
                "codex",
                "macbook",
            )
            .unwrap();

        let approval = task.human_approval.as_ref().unwrap();
        assert!(approval.required);
        assert_eq!(approval.approved_by.as_deref(), Some("codex"));
        assert!(approval.approved_at.is_some());
    }

    #[test]
    fn merge_releases_locks_and_unblocks_dependents() {
        let (_tmp, protocol) = fixture();
        protocol
            .register(
                RegisterTaskRequest {
                    issue: 1,
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
            .register(
                RegisterTaskRequest {
                    issue: 2,
                    task_id: "phase-b".into(),
                    title: "Phase B".into(),
                    dependencies: vec!["phase-a".into()],
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
                    approve: false,
                },
                "codex",
                "macbook",
            )
            .unwrap();
        protocol
            .assign("phase-a", "codex", "macbook", &[String::from("src/a.rs")])
            .unwrap();
        protocol
            .record_branch("phase-a", "feature/issue-1-phase-a", "codex", "macbook")
            .unwrap();
        protocol
            .record_pr("phase-a", 12, "codex", "macbook")
            .unwrap();

        let mut snapshot = protocol.snapshot_store.load().unwrap();
        let version = snapshot.version;
        let dependent = snapshot.get_task_mut("phase-b").unwrap();
        dependent.current_state = TaskState::Blocked;
        protocol.snapshot_store.save(&snapshot, version).unwrap();

        protocol
            .record_merge(
                "phase-a",
                "0123456789abcdef0123456789abcdef01234567",
                "codex",
                "macbook",
            )
            .unwrap();

        assert!(protocol.locks().unwrap().is_empty());
        let status = protocol.status(Some("phase-b")).unwrap();
        match status {
            StatusReport::Task(task) => assert_eq!(task.current_state, TaskState::Pending),
            StatusReport::Snapshot(_) => panic!("expected task status"),
        }
    }

    #[test]
    fn record_branch_rejects_invalid_branch_names() {
        let (_tmp, protocol) = fixture();
        protocol
            .register(
                RegisterTaskRequest {
                    issue: 1,
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

        let err = protocol
            .record_branch("phase-a", "bad-name", "codex", "macbook")
            .unwrap_err();

        assert!(matches!(err, ProtocolError::GateRejected(_)));
        assert!(err.to_string().contains("invalid branch name"));
    }

    #[test]
    fn assign_returns_dependency_blocked_when_hard_dependency_is_unresolved() {
        let (_tmp, protocol) = fixture();
        protocol
            .register(
                RegisterTaskRequest {
                    issue: 1,
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
            .register(
                RegisterTaskRequest {
                    issue: 2,
                    task_id: "phase-b".into(),
                    title: "Phase B".into(),
                    dependencies: vec!["phase-a".into()],
                    soft_dependencies: vec![],
                    priority: 0,
                    completion_mode: CompletionMode::GithubPr,
                },
                "codex",
                "macbook",
            )
            .unwrap();

        let err = protocol
            .assign(
                "phase-b",
                "codex",
                "macbook",
                &[String::from("crates/miyabi-core/src/protocol.rs")],
            )
            .unwrap_err();

        assert!(matches!(err, ProtocolError::DependencyBlocked(_)));
    }

    #[test]
    fn record_merge_treats_invalid_sha_as_gate_rejection() {
        let (_tmp, protocol) = fixture();
        protocol
            .register(
                RegisterTaskRequest {
                    issue: 1,
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

        let err = protocol
            .record_merge("phase-a", "not-a-valid-sha", "codex", "macbook")
            .unwrap_err();

        assert!(matches!(err, ProtocolError::GateRejected(_)));
    }
}
