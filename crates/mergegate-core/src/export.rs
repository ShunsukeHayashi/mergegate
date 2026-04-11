use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::store::{ExecutionTask, TasksSnapshot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportFilter {
    pub state: Option<String>,
    pub risk_level: Option<String>,
    pub since: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
struct ExportedTask<'a> {
    id: &'a str,
    title: &'a str,
    state: String,
    impact_risk: Option<String>,
    branch: Option<&'a str>,
    created_at: DateTime<Utc>,
}

pub fn export_json(snapshot: &TasksSnapshot, filter: Option<ExportFilter>) -> String {
    let tasks: Vec<ExportedTask<'_>> = filtered_tasks(snapshot, filter.as_ref())
        .into_iter()
        .map(ExportedTask::from_task)
        .collect();

    serde_json::to_string_pretty(&tasks).expect("task export should serialize")
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

impl<'a> ExportedTask<'a> {
    fn from_task(task: &'a ExecutionTask) -> Self {
        Self {
            id: &task.id,
            title: &task.title,
            state: task_state(task),
            impact_risk: task_risk(task),
            branch: task.branch_name.as_deref(),
            created_at: task.created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{export_json, ExportFilter};
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
}
