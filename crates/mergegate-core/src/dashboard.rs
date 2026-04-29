use std::collections::HashMap;

use crate::export::{export_payload, ExportFilter, ExportedTasksPayload};
use crate::protocol::{
    DagReport, DeterministicExecutionProtocol, DispatchableReport, StatusReport,
};
use crate::stats::{compute_stats, TaskStats};
use crate::store::{ContextAttachment, ExecutionTask, FileLockEntry, TasksSnapshot};
use crate::validate::{validate_snapshot, ValidationJsonReport};
use serde::{Deserialize, Serialize};

pub type DashboardTasksResponse = ExportedTasksPayload;
pub type DashboardStatusResponse = TasksSnapshot;
pub type DashboardStatsResponse = TaskStats;
pub type DashboardValidateResponse = ValidationJsonReport;
pub type DashboardLocksResponse = HashMap<String, FileLockEntry>;
pub type DashboardDagResponse = DagReport;
pub type DashboardDispatchableResponse = DispatchableReport;
pub type DashboardTaskDetailResponse = ExecutionTask;

const SPRINT_PLAN_LABELS: &[&str] = &[
    "Sprint Goal",
    "Primary",
    "Secondary",
    "Stretch",
    "Definition of Done",
    "DoD",
    "Out of scope",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DashboardSprintFocusResponse {
    pub plan: Option<ContextAttachment>,
    pub primary: Option<ExecutionTask>,
    pub secondary: Option<ExecutionTask>,
    pub stretch: Option<ExecutionTask>,
}

pub fn tasks_response(
    snapshot: &TasksSnapshot,
    filter: Option<&ExportFilter>,
) -> DashboardTasksResponse {
    export_payload(snapshot, filter)
}

pub fn status_response(snapshot: &TasksSnapshot) -> DashboardStatusResponse {
    snapshot.clone()
}

pub fn stats_response(snapshot: &TasksSnapshot) -> DashboardStatsResponse {
    compute_stats(snapshot)
}

pub fn validate_response(snapshot: &TasksSnapshot) -> DashboardValidateResponse {
    validate_snapshot(snapshot).to_json_report()
}

pub fn locks_response(
    protocol: &DeterministicExecutionProtocol,
) -> crate::protocol::ProtocolResult<DashboardLocksResponse> {
    protocol.locks()
}

pub fn dag_response(
    protocol: &DeterministicExecutionProtocol,
) -> crate::protocol::ProtocolResult<DashboardDagResponse> {
    protocol.dag()
}

pub fn dispatchable_response(
    protocol: &DeterministicExecutionProtocol,
) -> crate::protocol::ProtocolResult<DashboardDispatchableResponse> {
    protocol.dispatchable()
}

pub fn task_detail_response(
    protocol: &DeterministicExecutionProtocol,
    task_id: &str,
) -> crate::protocol::ProtocolResult<Option<DashboardTaskDetailResponse>> {
    match protocol.status(Some(task_id))? {
        StatusReport::Task(task) => Ok(Some(*task)),
        StatusReport::Snapshot(_) => Ok(None),
    }
}

pub fn sprint_focus_response(snapshot: &TasksSnapshot) -> DashboardSprintFocusResponse {
    let Some((plan_owner, plan)) = sprint_plan_attachment(snapshot) else {
        return DashboardSprintFocusResponse {
            plan: None,
            primary: None,
            secondary: None,
            stretch: None,
        };
    };

    DashboardSprintFocusResponse {
        plan: Some(plan.clone()),
        primary: task_after_label(snapshot, &plan.content, "Primary")
            .or_else(|| task_after_label(snapshot, &plan.content, "Sprint Goal"))
            .or_else(|| Some(plan_owner.clone())),
        secondary: task_after_label(snapshot, &plan.content, "Secondary"),
        stretch: task_after_label(snapshot, &plan.content, "Stretch"),
    }
}

fn sprint_plan_attachment(
    snapshot: &TasksSnapshot,
) -> Option<(&ExecutionTask, &ContextAttachment)> {
    snapshot.tasks.iter().find_map(|task| {
        task.context_attachments
            .iter()
            .find(|attachment| attachment.attachment_type == "sprint_plan")
            .map(|attachment| (task, attachment))
    })
}

fn task_after_label(snapshot: &TasksSnapshot, content: &str, label: &str) -> Option<ExecutionTask> {
    let lower_content = content.to_ascii_lowercase();
    let needle = format!("{}:", label.to_ascii_lowercase());
    let start = lower_content.find(&needle)? + needle.len();
    let section_len = SPRINT_PLAN_LABELS
        .iter()
        .filter(|candidate| !candidate.eq_ignore_ascii_case(label))
        .filter_map(|candidate| {
            lower_content[start..].find(&format!("{}:", candidate.to_ascii_lowercase()))
        })
        .min()
        .unwrap_or_else(|| lower_content[start..].len());
    let lower_section = &lower_content[start..start + section_len];

    snapshot
        .tasks
        .iter()
        .filter_map(|task| {
            lower_section
                .find(&task.id.to_ascii_lowercase())
                .map(|position| (position, task))
        })
        .min_by(|(left_position, left_task), (right_position, right_task)| {
            left_position
                .cmp(right_position)
                .then_with(|| right_task.id.len().cmp(&left_task.id.len()))
        })
        .map(|(_, task)| task.clone())
}

#[cfg(test)]
mod tests {
    use crate::protocol::{DeterministicExecutionProtocol, RegisterTaskRequest};
    use crate::store::{CompletionMode, ContextAttachment, TaskState, TasksSnapshot};
    use chrono::Utc;

    use super::{
        dag_response, dispatchable_response, sprint_focus_response, stats_response,
        task_detail_response, tasks_response, validate_response,
    };

    #[test]
    fn dashboard_payload_helpers_reuse_snapshot_contracts() {
        let snapshot = TasksSnapshot::default();

        let tasks = tasks_response(&snapshot, None);
        let stats = stats_response(&snapshot);
        let validation = validate_response(&snapshot);

        assert_eq!(tasks.task_count, 0);
        assert_eq!(stats.total, 0);
        assert_eq!(validation.issue_count, 0);
    }

    #[test]
    fn dashboard_protocol_helpers_return_task_detail_and_dag() {
        let tempdir = tempfile::tempdir().unwrap();
        let store_path = tempdir.path().join("project_memory/tasks.json");
        std::fs::create_dir_all(store_path.parent().unwrap()).unwrap();
        std::fs::write(
            &store_path,
            serde_json::to_vec_pretty(&TasksSnapshot::default()).unwrap(),
        )
        .unwrap();
        let protocol = DeterministicExecutionProtocol::from_store_path(store_path);

        protocol
            .register(
                RegisterTaskRequest {
                    issue: 0,
                    task_id: "task-a".to_string(),
                    title: "Task A".to_string(),
                    dependencies: Vec::new(),
                    soft_dependencies: Vec::new(),
                    priority: 1,
                    completion_mode: CompletionMode::Manual,
                },
                "tester",
                "node",
            )
            .unwrap();

        let task = task_detail_response(&protocol, "task-a")
            .unwrap()
            .expect("task exists");
        let dispatchable = dispatchable_response(&protocol).unwrap();
        let dag = dag_response(&protocol).unwrap();

        assert_eq!(task.current_state, TaskState::Pending);
        assert_eq!(dispatchable.count, 1);
        assert_eq!(dispatchable.task_ids, vec!["task-a".to_string()]);
        assert_eq!(dag.levels, vec![vec!["task-a".to_string()]]);
    }

    #[test]
    fn sprint_focus_response_resolves_plan_tasks_without_fixed_issue_ids() {
        let mut primary = crate::store::ExecutionTask::new("lane-alpha", "Gate Overview");
        primary.context_attachments.push(ContextAttachment {
            attachment_type: "sprint_plan".to_string(),
            source: "planning-wizard://sprint/example".to_string(),
            content: "Sprint Goal: complete lane-alpha. Secondary: start lane-beta. Stretch: design lane-gamma."
                .to_string(),
            token_estimate: 12,
            attached_at: Utc::now(),
        });
        let prefix = crate::store::ExecutionTask::new("lane", "Short Prefix");
        let secondary = crate::store::ExecutionTask::new("lane-beta", "Task Ledger");
        let stretch = crate::store::ExecutionTask::new("lane-gamma", "Dependency Map");
        let snapshot = TasksSnapshot {
            tasks: vec![prefix, primary, secondary, stretch],
            ..TasksSnapshot::default()
        };

        let focus = sprint_focus_response(&snapshot);

        assert_eq!(
            focus
                .plan
                .as_ref()
                .map(|plan| plan.attachment_type.as_str()),
            Some("sprint_plan")
        );
        assert_eq!(
            focus.primary.as_ref().map(|task| task.id.as_str()),
            Some("lane-alpha")
        );
        assert_eq!(
            focus.secondary.as_ref().map(|task| task.id.as_str()),
            Some("lane-beta")
        );
        assert_eq!(
            focus.stretch.as_ref().map(|task| task.id.as_str()),
            Some("lane-gamma")
        );
    }

    #[test]
    fn sprint_focus_response_does_not_cross_into_later_lanes() {
        let mut primary = crate::store::ExecutionTask::new("lane-alpha", "Gate Overview");
        primary.context_attachments.push(ContextAttachment {
            attachment_type: "sprint_plan".to_string(),
            source: "planning-wizard://sprint/example".to_string(),
            content: "Sprint Goal: complete lane-alpha. Secondary: needs discovery only. Stretch: design lane-gamma. Out of scope: lane-beta."
                .to_string(),
            token_estimate: 18,
            attached_at: Utc::now(),
        });
        let secondary = crate::store::ExecutionTask::new("lane-beta", "Task Ledger");
        let stretch = crate::store::ExecutionTask::new("lane-gamma", "Dependency Map");
        let snapshot = TasksSnapshot {
            tasks: vec![primary, secondary, stretch],
            ..TasksSnapshot::default()
        };

        let focus = sprint_focus_response(&snapshot);

        assert_eq!(
            focus.primary.as_ref().map(|task| task.id.as_str()),
            Some("lane-alpha")
        );
        assert!(focus.secondary.is_none());
        assert_eq!(
            focus.stretch.as_ref().map(|task| task.id.as_str()),
            Some("lane-gamma")
        );
    }
}
