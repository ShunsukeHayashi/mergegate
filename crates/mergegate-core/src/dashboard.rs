use std::collections::HashMap;

use crate::export::{export_payload, ExportFilter, ExportedTasksPayload};
use crate::protocol::{DagReport, DeterministicExecutionProtocol, DispatchableReport, StatusReport};
use crate::stats::{compute_stats, TaskStats};
use crate::store::{ExecutionTask, FileLockEntry, TasksSnapshot};
use crate::validate::{validate_snapshot, ValidationJsonReport};

pub type DashboardTasksResponse = ExportedTasksPayload;
pub type DashboardStatusResponse = TasksSnapshot;
pub type DashboardStatsResponse = TaskStats;
pub type DashboardValidateResponse = ValidationJsonReport;
pub type DashboardLocksResponse = HashMap<String, FileLockEntry>;
pub type DashboardDagResponse = DagReport;
pub type DashboardDispatchableResponse = DispatchableReport;
pub type DashboardTaskDetailResponse = ExecutionTask;

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

#[cfg(test)]
mod tests {
    use crate::protocol::{DeterministicExecutionProtocol, RegisterTaskRequest};
    use crate::store::{CompletionMode, TaskState, TasksSnapshot};

    use super::{
        dag_response, dispatchable_response, stats_response, task_detail_response, tasks_response,
        validate_response,
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
        std::fs::write(&store_path, serde_json::to_vec_pretty(&TasksSnapshot::default()).unwrap())
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
}
