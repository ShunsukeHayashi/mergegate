//! Deterministic gate evaluation for task execution.

use crate::lock::LockConflict;
use crate::store::{
    CompletionMode, ExecutionTask, GitHubIssueState, GitHubPrState, ReviewDecision, TaskState,
    TasksSnapshot,
};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gate {
    Gate0,
    Gate1,
    Gate2,
    Gate3,
    Gate4,
    Gate5,
    Gate6,
    Gate7,
    Gate8,
}

impl Gate {
    pub fn label(self) -> &'static str {
        match self {
            Gate::Gate0 => "gate_0",
            Gate::Gate1 => "gate_1",
            Gate::Gate2 => "gate_2",
            Gate::Gate3 => "gate_3",
            Gate::Gate4 => "gate_4",
            Gate::Gate5 => "gate_5",
            Gate::Gate6 => "gate_6",
            Gate::Gate7 => "gate_7",
            Gate::Gate8 => "gate_8",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GateContext {
    pub lock_conflict: Option<LockConflict>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateReport {
    pub gate: Gate,
    pub success: bool,
    pub detail: String,
    pub duration: Duration,
}

pub fn evaluate_gate(
    gate: Gate,
    task: &ExecutionTask,
    snapshot: &TasksSnapshot,
    context: &GateContext,
) -> GateReport {
    let start = Instant::now();
    let (success, detail) = match gate {
        Gate::Gate0 => (true, "task declared".to_string()),
        Gate::Gate1 => (
            matches!(task.current_state, TaskState::Draft | TaskState::Pending),
            format!("state is {:?}", task.current_state),
        ),
        Gate::Gate2 => {
            let blocked_by: Vec<String> = task
                .dependencies
                .iter()
                .filter_map(|dependency| {
                    snapshot
                        .get_task(dependency)
                        .filter(|dep| {
                            !matches!(dep.current_state, TaskState::Done | TaskState::Merged)
                        })
                        .map(|_| dependency.clone())
                })
                .collect();
            (
                blocked_by.is_empty(),
                if blocked_by.is_empty() {
                    "all hard dependencies resolved".to_string()
                } else {
                    format!("blocked by dependencies: {}", blocked_by.join(", "))
                },
            )
        }
        Gate::Gate3 => {
            let has_impact = task.impact.is_some();
            let approval_ok = task.human_approval.as_ref().map_or(true, |approval| {
                !approval.required || approval.approved_by.is_some()
            });
            (
                has_impact && approval_ok,
                if !has_impact {
                    "missing impact analysis".to_string()
                } else if !approval_ok {
                    "human approval required".to_string()
                } else {
                    "impact and approval satisfied".to_string()
                },
            )
        }
        Gate::Gate4 => {
            let conflict = context
                .lock_conflict
                .as_ref()
                .filter(|conflict| conflict.conflicting);
            (
                conflict.is_none(),
                conflict
                    .map(|conflict| {
                        format!(
                            "lock conflict held by {}",
                            conflict.held_by.as_deref().unwrap_or("unknown")
                        )
                    })
                    .unwrap_or_else(|| "lock window available".to_string()),
            )
        }
        Gate::Gate5 => (
            task.branch_name.is_some(),
            task.branch_name
                .clone()
                .unwrap_or_else(|| "missing branch_name".to_string()),
        ),
        Gate::Gate6 => {
            let ok = task.github_evidence.as_ref().is_some_and(|evidence| {
                evidence.pr_number > 0
                    && !evidence.pr_head_ref.is_empty()
                    && matches!(
                        evidence.review_decision,
                        Some(ReviewDecision::Approved | ReviewDecision::ReviewRequired)
                    )
            });
            (
                ok,
                if ok {
                    "pull request evidence present".to_string()
                } else {
                    "missing verified pull request evidence".to_string()
                },
            )
        }
        Gate::Gate7 => {
            let ok = task.github_evidence.as_ref().is_some_and(|evidence| {
                evidence.pr_state == GitHubPrState::Merged && evidence.merge_commit_sha.is_some()
            });
            (
                ok,
                if ok {
                    "merge verified".to_string()
                } else {
                    "merge not verified".to_string()
                },
            )
        }
        Gate::Gate8 => {
            let ok = match task.completion_mode {
                CompletionMode::GithubPr => {
                    task.github_evidence.as_ref().is_some_and(|evidence| {
                        evidence.issue_state == GitHubIssueState::Closed
                    })
                }
                CompletionMode::Manual | CompletionMode::ExternalOp => true,
            };
            (
                ok,
                if ok {
                    "completion evidence satisfied".to_string()
                } else {
                    "issue still open".to_string()
                },
            )
        }
    };

    GateReport {
        gate,
        success,
        detail,
        duration: start.elapsed(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{
        CompletionMode, ExecutionTask, GitHubEvidence, ImpactRiskLevel, ReviewDecision, TaskImpact,
        TaskState, TasksSnapshot,
    };
    use chrono::Utc;

    fn task(id: &str) -> ExecutionTask {
        let mut task = ExecutionTask::new(id, "gate test");
        task.current_state = TaskState::Pending;
        task
    }

    #[test]
    fn gate_2_blocks_on_incomplete_dependency() {
        let mut pending = task("child");
        pending.dependencies.push("parent".into());

        let mut snapshot = TasksSnapshot::default();
        snapshot.upsert_task(pending.clone());
        let mut parent = task("parent");
        parent.current_state = TaskState::Implementing;
        snapshot.upsert_task(parent);

        let report = evaluate_gate(Gate::Gate2, &pending, &snapshot, &GateContext::default());
        assert!(!report.success);
        assert!(report.detail.contains("parent"));
    }

    #[test]
    fn gate_3_requires_impact_and_human_approval_when_flagged() {
        let mut snapshot = TasksSnapshot::default();
        let mut gated = task("phase-a");
        gated.impact = Some(TaskImpact {
            risk_level: ImpactRiskLevel::Low,
            affected_symbols: 1,
            depth1: vec!["create_orchestrator".into()],
            analyzed_at: Utc::now(),
            analyzed_commit: None,
            input_hash: None,
        });
        gated.human_approval = Some(crate::store::HumanApproval {
            required: true,
            approved_by: None,
            approved_at: None,
            reason: Some("touches orchestration".into()),
        });
        snapshot.upsert_task(gated.clone());

        let report = evaluate_gate(Gate::Gate3, &gated, &snapshot, &GateContext::default());
        assert!(!report.success);
        assert_eq!(report.detail, "human approval required");
    }

    #[test]
    fn merge_and_close_gates_pass_with_verified_evidence() {
        let mut task = task("phase-a");
        task.current_state = TaskState::Reviewing;
        task.completion_mode = CompletionMode::GithubPr;
        task.branch_name = Some("feature/phase-a".into());
        task.github_evidence = Some(GitHubEvidence {
            pr_number: 12,
            pr_head_ref: "feature/phase-a".into(),
            pr_state: GitHubPrState::Merged,
            merge_commit_sha: Some("0123456789abcdef0123456789abcdef01234567".into()),
            merged_at: Some(Utc::now()),
            review_decision: Some(ReviewDecision::Approved),
            issue_state: GitHubIssueState::Closed,
            issue_closed_by_pr: true,
        });
        let mut snapshot = TasksSnapshot::default();
        snapshot.upsert_task(task.clone());

        assert!(evaluate_gate(Gate::Gate6, &task, &snapshot, &GateContext::default()).success);
        assert!(evaluate_gate(Gate::Gate7, &task, &snapshot, &GateContext::default()).success);
        assert!(evaluate_gate(Gate::Gate8, &task, &snapshot, &GateContext::default()).success);
    }
}
