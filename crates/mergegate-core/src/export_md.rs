use crate::export::ExportFilter;
use crate::store::{ExecutionTask, TaskState, TasksSnapshot};

const TITLE_LIMIT: usize = 48;

pub fn export_markdown(snapshot: &TasksSnapshot, filter: Option<&ExportFilter>) -> String {
    let mut lines = vec![
        "| ID | Title | State | Risk | Branch | Created |".to_string(),
        "| --- | --- | --- | --- | --- | --- |".to_string(),
    ];

    let tasks = crate::export::filtered_tasks(snapshot, filter);

    for task in &tasks {
        lines.push(format!(
            "| {} | {} | {} | {} | {} | {} |",
            escape_cell(&task.id),
            escape_cell(&truncate_title(&task.title)),
            format_state(task),
            format_risk(task),
            escape_cell(task.branch_name.as_deref().unwrap_or("-")),
            task.created_at.format("%Y-%m-%d"),
        ));
    }

    let completed = tasks.iter().filter(|task| is_completed(task)).count();
    let active = tasks.iter().filter(|task| is_active(task)).count();
    let waiting = tasks.iter().filter(|task| is_waiting(task)).count();

    lines.push(String::new());
    lines.push(format!(
        "Total: {} tasks ({} completed, {} active, {} waiting)",
        tasks.len(),
        completed,
        active,
        waiting
    ));

    lines.join("\n")
}

fn truncate_title(title: &str) -> String {
    let mut chars = title.chars();
    if title.chars().count() <= TITLE_LIMIT {
        return title.to_string();
    }

    let truncated: String = chars.by_ref().take(TITLE_LIMIT - 3).collect();
    format!("{truncated}...")
}

fn format_state(task: &ExecutionTask) -> String {
    match serde_json::to_value(task.current_state)
        .expect("task state should serialize")
        .as_str()
        .expect("task state should serialize as string")
    {
        "done" => "✅ Done".to_string(),
        "merged" => "🔀 Merged".to_string(),
        "implementing" => "🔧 Implementing".to_string(),
        "pending" => "⏳ Pending".to_string(),
        other => title_case(other),
    }
}

fn format_risk(task: &ExecutionTask) -> String {
    task.impact
        .as_ref()
        .map(|impact| {
            serde_json::to_value(impact.risk_level)
                .expect("impact risk should serialize")
                .as_str()
                .expect("impact risk should serialize as string")
                .to_string()
        })
        .unwrap_or_else(|| "-".to_string())
}

fn is_completed(task: &ExecutionTask) -> bool {
    matches!(task.current_state, TaskState::Done | TaskState::Merged)
}

fn is_active(task: &ExecutionTask) -> bool {
    matches!(
        task.current_state,
        TaskState::Implementing
            | TaskState::Analyzing
            | TaskState::Reviewing
            | TaskState::Deploying
    )
}

fn is_waiting(task: &ExecutionTask) -> bool {
    matches!(
        task.current_state,
        TaskState::Draft | TaskState::Pending | TaskState::Blocked | TaskState::AwaitingGithubSync
    )
}

fn title_case(value: &str) -> String {
    value
        .split('_')
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                Some(first) => {
                    let mut out = first.to_uppercase().collect::<String>();
                    out.push_str(chars.as_str());
                    out
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn escape_cell(value: &str) -> String {
    value.replace('|', "\\|")
}

#[cfg(test)]
mod tests {
    use super::export_markdown;
    use crate::export::ExportFilter;
    use crate::store::{ExecutionTask, ImpactRiskLevel, TaskImpact, TaskState, TasksSnapshot};
    use chrono::{Duration, TimeZone, Utc};

    fn sample_task(
        id: &str,
        title: &str,
        state: TaskState,
        risk_level: Option<ImpactRiskLevel>,
    ) -> ExecutionTask {
        let created_at = Utc.with_ymd_and_hms(2026, 4, 10, 12, 0, 0).unwrap();
        let mut task = ExecutionTask::new(id, title);
        task.current_state = state;
        task.branch_name = Some(format!("branch/{id}"));
        task.created_at = created_at;
        task.updated_at = created_at + Duration::minutes(5);
        task.impact = risk_level.map(|risk_level| TaskImpact {
            risk_level,
            affected_symbols: 1,
            depth1: vec!["src/lib.rs".to_string()],
            analyzed_at: created_at,
            analyzed_commit: Some("abc123".to_string()),
            input_hash: Some("hash".to_string()),
        });
        task
    }

    fn empty_snapshot() -> TasksSnapshot {
        TasksSnapshot {
            version: 1,
            generated_at: Utc.with_ymd_and_hms(2026, 4, 10, 12, 0, 0).unwrap(),
            generated_from_event_id: Some("event-1".to_string()),
            tasks: Vec::new(),
            file_locks: Default::default(),
        }
    }

    #[test]
    fn exports_full_table() {
        let mut snapshot = empty_snapshot();
        snapshot.tasks = vec![
            sample_task(
                "task-1",
                "Implement markdown export",
                TaskState::Implementing,
                Some(ImpactRiskLevel::Medium),
            ),
            sample_task(
                "task-2",
                "Merge snapshot writer",
                TaskState::Merged,
                Some(ImpactRiskLevel::High),
            ),
        ];

        let markdown = export_markdown(&snapshot, None);

        assert!(markdown.contains("| ID | Title | State | Risk | Branch | Created |"));
        assert!(markdown.contains("| task-1 | Implement markdown export | 🔧 Implementing | MEDIUM | branch/task-1 | 2026-04-10 |"));
        assert!(markdown.contains(
            "| task-2 | Merge snapshot writer | 🔀 Merged | HIGH | branch/task-2 | 2026-04-10 |"
        ));
        assert!(markdown.ends_with("Total: 2 tasks (1 completed, 1 active, 0 waiting)"));
    }

    #[test]
    fn exports_empty_snapshot() {
        let markdown = export_markdown(&empty_snapshot(), None);

        assert_eq!(
            markdown,
            "| ID | Title | State | Risk | Branch | Created |\n| --- | --- | --- | --- | --- | --- |\n\nTotal: 0 tasks (0 completed, 0 active, 0 waiting)"
        );
    }

    #[test]
    fn exports_single_task() {
        let mut snapshot = empty_snapshot();
        snapshot
            .tasks
            .push(sample_task("task-1", "Ship feature", TaskState::Done, None));

        let markdown = export_markdown(&snapshot, None);

        assert!(markdown
            .contains("| task-1 | Ship feature | ✅ Done | - | branch/task-1 | 2026-04-10 |"));
        assert!(markdown.ends_with("Total: 1 tasks (1 completed, 0 active, 0 waiting)"));
    }

    #[test]
    fn summarizes_mixed_states() {
        let mut snapshot = empty_snapshot();
        snapshot.tasks = vec![
            sample_task("done", "Done task", TaskState::Done, None),
            sample_task("merged", "Merged task", TaskState::Merged, None),
            sample_task("impl", "Implementing task", TaskState::Implementing, None),
            sample_task("pending", "Pending task", TaskState::Pending, None),
            sample_task("draft", "Draft task", TaskState::Draft, None),
        ];

        let markdown = export_markdown(&snapshot, None);

        assert!(markdown.contains("| done | Done task | ✅ Done | - | branch/done | 2026-04-10 |"));
        assert!(markdown
            .contains("| merged | Merged task | 🔀 Merged | - | branch/merged | 2026-04-10 |"));
        assert!(markdown.contains(
            "| impl | Implementing task | 🔧 Implementing | - | branch/impl | 2026-04-10 |"
        ));
        assert!(markdown
            .contains("| pending | Pending task | ⏳ Pending | - | branch/pending | 2026-04-10 |"));
        assert!(markdown.contains("| draft | Draft task | Draft | - | branch/draft | 2026-04-10 |"));
        assert!(markdown.ends_with("Total: 5 tasks (2 completed, 1 active, 2 waiting)"));
    }

    #[test]
    fn truncates_long_titles() {
        let mut snapshot = empty_snapshot();
        snapshot.tasks.push(sample_task(
            "task-1",
            "This is an intentionally long title that should be truncated in markdown output",
            TaskState::Pending,
            Some(ImpactRiskLevel::Low),
        ));

        let markdown = export_markdown(&snapshot, None);

        assert!(markdown.contains("This is an intentionally long title that shou..."));
        assert!(!markdown.contains(
            "This is an intentionally long title that should be truncated in markdown output"
        ));
    }

    #[test]
    fn filters_markdown_output() {
        let mut snapshot = empty_snapshot();
        snapshot.tasks = vec![
            sample_task(
                "task-1",
                "Implement markdown export",
                TaskState::Implementing,
                Some(ImpactRiskLevel::Medium),
            ),
            sample_task(
                "task-2",
                "Ship release",
                TaskState::Done,
                Some(ImpactRiskLevel::High),
            ),
        ];

        let markdown = export_markdown(
            &snapshot,
            Some(&ExportFilter {
                state: Some("implementing".to_string()),
                risk_level: None,
                since: None,
            }),
        );

        assert!(markdown.contains("| task-1 | Implement markdown export |"));
        assert!(!markdown.contains("| task-2 | Ship release |"));
        assert!(markdown.ends_with("Total: 1 tasks (0 completed, 1 active, 0 waiting)"));
    }
}
