use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::store::{ExecutionTask, TasksSnapshot};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportFilter {
    pub state: Option<String>,
    pub risk_level: Option<String>,
    pub since: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportedTask {
    pub id: String,
    pub title: String,
    pub state: String,
    pub current_state: String,
    pub impact_risk: Option<String>,
    pub branch: Option<String>,
    pub dependencies: Vec<String>,
    pub priority: u32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportedTasksPayload {
    pub version: u64,
    pub generated_at: DateTime<Utc>,
    pub generated_from_event_id: Option<String>,
    pub task_count: usize,
    pub tasks: Vec<ExportedTask>,
}

pub fn export_json(snapshot: &TasksSnapshot, filter: Option<ExportFilter>) -> String {
    let tasks = export_tasks(snapshot, filter.as_ref());

    serde_json::to_string_pretty(&tasks).expect("task export should serialize")
}

pub fn export_tasks(snapshot: &TasksSnapshot, filter: Option<&ExportFilter>) -> Vec<ExportedTask> {
    filtered_tasks(snapshot, filter)
        .into_iter()
        .map(ExportedTask::from_task)
        .collect()
}

pub fn export_payload(
    snapshot: &TasksSnapshot,
    filter: Option<&ExportFilter>,
) -> ExportedTasksPayload {
    let tasks = export_tasks(snapshot, filter);
    ExportedTasksPayload {
        version: snapshot.version,
        generated_at: snapshot.generated_at,
        generated_from_event_id: snapshot.generated_from_event_id.clone(),
        task_count: tasks.len(),
        tasks,
    }
}

pub fn filtered_tasks<'a>(
    snapshot: &'a TasksSnapshot,
    filter: Option<&ExportFilter>,
) -> Vec<&'a ExecutionTask> {
    snapshot
        .tasks
        .iter()
        .filter(|task| matches_filter(task, filter))
        .collect()
}

pub fn matches_filter(task: &ExecutionTask, filter: Option<&ExportFilter>) -> bool {
    let Some(filter) = filter else {
        return true;
    };

    if let Some(expected_state) = filter.state.as_deref() {
        if task_state(task) != expected_state {
            return false;
        }
    }

    if let Some(expected_risk) = filter.risk_level.as_deref() {
        if task_risk(task).as_deref() != Some(expected_risk) {
            return false;
        }
    }

    if let Some(since) = filter.since {
        if task.created_at < since {
            return false;
        }
    }

    true
}

fn task_state(task: &ExecutionTask) -> String {
    serde_json::to_value(task.current_state)
        .expect("task state should serialize")
        .as_str()
        .expect("task state should serialize as string")
        .to_owned()
}

fn task_risk(task: &ExecutionTask) -> Option<String> {
    task.impact.as_ref().map(|impact| {
        serde_json::to_value(impact.risk_level)
            .expect("impact risk should serialize")
            .as_str()
            .expect("impact risk should serialize as string")
            .to_owned()
    })
}

impl ExportedTask {
    fn from_task(task: &ExecutionTask) -> Self {
        let state = task_state(task);
        Self {
            id: task.id.clone(),
            title: task.title.clone(),
            state: state.clone(),
            current_state: state,
            impact_risk: task_risk(task),
            branch: task.branch_name.clone(),
            dependencies: task.dependencies.clone(),
            priority: task.priority,
            created_at: task.created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{export_json, export_payload, ExportFilter};
    use crate::store::{ExecutionTask, ImpactRiskLevel, TaskImpact, TaskState, TasksSnapshot};
    use chrono::{Duration, TimeZone, Utc};
    use serde_json::Value;

    fn sample_task(
        id: &str,
        state: TaskState,
        risk_level: Option<ImpactRiskLevel>,
        created_at: chrono::DateTime<Utc>,
    ) -> ExecutionTask {
        let mut task = ExecutionTask::new(id, format!("Task {id}"));
        task.current_state = state;
        task.branch_name = Some(format!("branch/{id}"));
        task.created_at = created_at;
        task.updated_at = created_at;
        task.impact = risk_level.map(|risk_level| TaskImpact {
            risk_level,
            affected_symbols: 2,
            depth1: vec!["src/lib.rs".to_string()],
            analyzed_at: created_at,
            analyzed_commit: Some("abc123".to_string()),
            input_hash: Some("hash".to_string()),
        });
        task
    }

    fn sample_snapshot() -> TasksSnapshot {
        let base = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();
        TasksSnapshot {
            version: 1,
            generated_at: base + Duration::days(10),
            generated_from_event_id: Some("event-1".to_string()),
            tasks: vec![
                sample_task("task-1", TaskState::Draft, Some(ImpactRiskLevel::Low), base),
                sample_task(
                    "task-2",
                    TaskState::Implementing,
                    Some(ImpactRiskLevel::High),
                    base + Duration::days(1),
                ),
                sample_task(
                    "task-3",
                    TaskState::Done,
                    Some(ImpactRiskLevel::Critical),
                    base + Duration::days(2),
                ),
            ],
            file_locks: Default::default(),
        }
    }

    fn exported_ids(json: &str) -> Vec<String> {
        serde_json::from_str::<Vec<Value>>(json)
            .expect("valid json")
            .into_iter()
            .map(|task| {
                task.get("id")
                    .and_then(Value::as_str)
                    .expect("id field")
                    .to_string()
            })
            .collect()
    }

    #[test]
    fn exports_all_tasks_without_filter() {
        let snapshot = sample_snapshot();

        let json = export_json(&snapshot, None);
        let tasks = serde_json::from_str::<Vec<Value>>(&json).expect("valid json");

        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0]["id"], "task-1");
        assert_eq!(tasks[0]["state"], "draft");
        assert_eq!(tasks[0]["current_state"], "draft");
        assert_eq!(tasks[0]["impact_risk"], "LOW");
        assert_eq!(tasks[0]["branch"], "branch/task-1");
        assert_eq!(tasks[0]["created_at"], "2026-04-01T00:00:00Z");
    }

    #[test]
    fn filters_by_state() {
        let snapshot = sample_snapshot();

        let json = export_json(
            &snapshot,
            Some(ExportFilter {
                state: Some("implementing".to_string()),
                risk_level: None,
                since: None,
            }),
        );

        assert_eq!(exported_ids(&json), vec!["task-2"]);
    }

    #[test]
    fn filters_by_risk_level() {
        let snapshot = sample_snapshot();

        let json = export_json(
            &snapshot,
            Some(ExportFilter {
                state: None,
                risk_level: Some("CRITICAL".to_string()),
                since: None,
            }),
        );

        assert_eq!(exported_ids(&json), vec!["task-3"]);
    }

    #[test]
    fn filters_by_creation_date() {
        let snapshot = sample_snapshot();
        let since = Utc.with_ymd_and_hms(2026, 4, 2, 0, 0, 0).unwrap();

        let json = export_json(
            &snapshot,
            Some(ExportFilter {
                state: None,
                risk_level: None,
                since: Some(since),
            }),
        );

        assert_eq!(exported_ids(&json), vec!["task-2", "task-3"]);
    }

    #[test]
    fn returns_empty_array_when_no_tasks_match() {
        let snapshot = sample_snapshot();

        let json = export_json(
            &snapshot,
            Some(ExportFilter {
                state: Some("blocked".to_string()),
                risk_level: Some("LOW".to_string()),
                since: None,
            }),
        );

        assert_eq!(json, "[]");
    }

    #[test]
    fn exports_payload_with_snapshot_metadata() {
        let snapshot = sample_snapshot();

        let payload = export_payload(
            &snapshot,
            Some(&ExportFilter {
                state: Some("implementing".to_string()),
                risk_level: None,
                since: None,
            }),
        );

        assert_eq!(payload.version, 1);
        assert_eq!(payload.generated_at, snapshot.generated_at);
        assert_eq!(payload.generated_from_event_id.as_deref(), Some("event-1"));
        assert_eq!(payload.task_count, 1);
        assert_eq!(payload.tasks[0].id, "task-2");
        assert_eq!(payload.tasks[0].current_state, "implementing");
    }
}
