use crate::store::{ExecutionTask, TasksSnapshot};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskStats {
    pub total: usize,
    pub completed: usize,
    pub active: usize,
    pub waiting: usize,
    pub failed: usize,
    pub completion_rate_pct: f64,
    pub avg_lead_time_hours: Option<f64>,
    pub risk_distribution: HashMap<String, usize>,
}

pub fn compute_stats(snapshot: &TasksSnapshot) -> TaskStats {
    let total = snapshot.tasks.len();
    let completed = snapshot
        .tasks
        .iter()
        .filter(|task| is_completed(task))
        .count();
    let active = snapshot.tasks.iter().filter(|task| is_active(task)).count();
    let waiting = snapshot
        .tasks
        .iter()
        .filter(|task| is_waiting(task))
        .count();
    let failed = snapshot.tasks.iter().filter(|task| is_failed(task)).count();

    let completion_rate_pct = if total == 0 {
        0.0
    } else {
        (completed as f64 / total as f64) * 100.0
    };

    let completed_lead_times: Vec<f64> = snapshot
        .tasks
        .iter()
        .filter(|task| is_completed(task))
        .map(task_lead_time_hours)
        .collect();

    let avg_lead_time_hours = if completed_lead_times.is_empty() {
        None
    } else {
        Some(completed_lead_times.iter().sum::<f64>() / completed_lead_times.len() as f64)
    };

    let mut risk_distribution = HashMap::from([
        ("low".to_string(), 0usize),
        ("medium".to_string(), 0usize),
        ("high".to_string(), 0usize),
        ("critical".to_string(), 0usize),
    ]);

    for task in &snapshot.tasks {
        if let Some(impact) = &task.impact {
            let key = match impact.risk_level {
                crate::store::ImpactRiskLevel::Low => "low",
                crate::store::ImpactRiskLevel::Medium => "medium",
                crate::store::ImpactRiskLevel::High => "high",
                crate::store::ImpactRiskLevel::Critical => "critical",
            };
            *risk_distribution.entry(key.to_string()).or_insert(0) += 1;
        }
    }

    TaskStats {
        total,
        completed,
        active,
        waiting,
        failed,
        completion_rate_pct,
        avg_lead_time_hours,
        risk_distribution,
    }
}

fn is_completed(task: &ExecutionTask) -> bool {
    matches!(
        task.current_state,
        crate::store::TaskState::Done | crate::store::TaskState::Merged
    )
}

fn is_active(task: &ExecutionTask) -> bool {
    matches!(
        task.current_state,
        crate::store::TaskState::Analyzing
            | crate::store::TaskState::Implementing
            | crate::store::TaskState::Reviewing
            | crate::store::TaskState::Deploying
    )
}

fn is_waiting(task: &ExecutionTask) -> bool {
    matches!(
        task.current_state,
        crate::store::TaskState::Draft
            | crate::store::TaskState::Pending
            | crate::store::TaskState::Blocked
            | crate::store::TaskState::AwaitingGithubSync
    )
}

fn is_failed(task: &ExecutionTask) -> bool {
    matches!(task.current_state, crate::store::TaskState::Failed)
}

fn task_lead_time_hours(task: &ExecutionTask) -> f64 {
    hours_between(task.created_at, task.updated_at)
}

fn hours_between(start: DateTime<Utc>, end: DateTime<Utc>) -> f64 {
    (end - start).num_seconds() as f64 / 3600.0
}

impl fmt::Display for TaskStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let avg_lead_time = self
            .avg_lead_time_hours
            .map(|hours| format!("{hours:.2}h"))
            .unwrap_or_else(|| "n/a".to_string());

        writeln!(f, "MergeGate Task Stats")?;
        writeln!(f, "  Total: {}", self.total)?;
        writeln!(
            f,
            "  Completed: {} ({:.1}%)",
            self.completed, self.completion_rate_pct
        )?;
        writeln!(f, "  Active: {}", self.active)?;
        writeln!(f, "  Waiting: {}", self.waiting)?;
        writeln!(f, "  Failed: {}", self.failed)?;
        writeln!(f, "  Avg lead time: {}", avg_lead_time)?;
        writeln!(
            f,
            "  Risk: low={} medium={} high={} critical={}",
            self.risk_distribution.get("low").copied().unwrap_or(0),
            self.risk_distribution.get("medium").copied().unwrap_or(0),
            self.risk_distribution.get("high").copied().unwrap_or(0),
            self.risk_distribution.get("critical").copied().unwrap_or(0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{compute_stats, TaskStats};
    use crate::store::{ExecutionTask, ImpactRiskLevel, TaskImpact, TaskState, TasksSnapshot};
    use chrono::{Duration, TimeZone, Utc};

    #[test]
    fn empty_snapshot_returns_zeroed_stats() {
        let stats = compute_stats(&TasksSnapshot::default());

        assert_eq!(
            stats,
            TaskStats {
                total: 0,
                completed: 0,
                active: 0,
                waiting: 0,
                failed: 0,
                completion_rate_pct: 0.0,
                avg_lead_time_hours: None,
                risk_distribution: [
                    ("low".to_string(), 0),
                    ("medium".to_string(), 0),
                    ("high".to_string(), 0),
                    ("critical".to_string(), 0),
                ]
                .into_iter()
                .collect(),
            }
        );
    }

    #[test]
    fn all_done_snapshot_has_full_completion_rate() {
        let snapshot = TasksSnapshot {
            tasks: vec![
                task("t1", TaskState::Done, 2, None),
                task("t2", TaskState::Done, 4, None),
            ],
            ..TasksSnapshot::default()
        };

        let stats = compute_stats(&snapshot);

        assert_eq!(stats.total, 2);
        assert_eq!(stats.completed, 2);
        assert_eq!(stats.active, 0);
        assert_eq!(stats.waiting, 0);
        assert_eq!(stats.failed, 0);
        assert!((stats.completion_rate_pct - 100.0).abs() < f64::EPSILON);
        assert_eq!(stats.avg_lead_time_hours, Some(3.0));
    }

    #[test]
    fn mixed_states_are_counted_by_bucket() {
        let snapshot = TasksSnapshot {
            tasks: vec![
                task("done", TaskState::Done, 1, None),
                task("active1", TaskState::Implementing, 1, None),
                task("active2", TaskState::Reviewing, 1, None),
                task("waiting1", TaskState::Pending, 1, None),
                task("waiting2", TaskState::Blocked, 1, None),
                task("failed", TaskState::Failed, 1, None),
            ],
            ..TasksSnapshot::default()
        };

        let stats = compute_stats(&snapshot);

        assert_eq!(stats.total, 6);
        assert_eq!(stats.completed, 1);
        assert_eq!(stats.active, 2);
        assert_eq!(stats.waiting, 2);
        assert_eq!(stats.failed, 1);
        assert!((stats.completion_rate_pct - (100.0 / 6.0)).abs() < 1e-9);
    }

    #[test]
    fn risk_distribution_counts_each_level() {
        let snapshot = TasksSnapshot {
            tasks: vec![
                task("low", TaskState::Pending, 1, Some(ImpactRiskLevel::Low)),
                task(
                    "medium",
                    TaskState::Pending,
                    1,
                    Some(ImpactRiskLevel::Medium),
                ),
                task("high", TaskState::Pending, 1, Some(ImpactRiskLevel::High)),
                task(
                    "critical",
                    TaskState::Pending,
                    1,
                    Some(ImpactRiskLevel::Critical),
                ),
                task("none", TaskState::Pending, 1, None),
            ],
            ..TasksSnapshot::default()
        };

        let stats = compute_stats(&snapshot);

        assert_eq!(stats.risk_distribution.get("low"), Some(&1));
        assert_eq!(stats.risk_distribution.get("medium"), Some(&1));
        assert_eq!(stats.risk_distribution.get("high"), Some(&1));
        assert_eq!(stats.risk_distribution.get("critical"), Some(&1));
    }

    #[test]
    fn average_lead_time_only_uses_completed_tasks() {
        let snapshot = TasksSnapshot {
            tasks: vec![
                task("done1", TaskState::Done, 2, None),
                task("done2", TaskState::Done, 4, None),
                task("active", TaskState::Implementing, 20, None),
            ],
            ..TasksSnapshot::default()
        };

        let stats = compute_stats(&snapshot);

        assert_eq!(stats.avg_lead_time_hours, Some(3.0));
    }

    #[test]
    fn merged_counts_as_completed_not_active() {
        let snapshot = TasksSnapshot {
            tasks: vec![task("merged", TaskState::Merged, 2, None)],
            ..TasksSnapshot::default()
        };

        let stats = compute_stats(&snapshot);

        assert_eq!(stats.completed, 1);
        assert_eq!(stats.active, 0);
        assert_eq!(stats.waiting, 0);
        assert_eq!(stats.failed, 0);
    }

    #[test]
    fn display_includes_terminal_friendly_summary() {
        let snapshot = TasksSnapshot {
            tasks: vec![task(
                "done",
                TaskState::Done,
                2,
                Some(ImpactRiskLevel::High),
            )],
            ..TasksSnapshot::default()
        };

        let rendered = compute_stats(&snapshot).to_string();

        assert!(rendered.contains("MergeGate Task Stats"));
        assert!(rendered.contains("Completed: 1 (100.0%)"));
        assert!(rendered.contains("Failed: 0"));
        assert!(rendered.contains("Avg lead time: 2.00h"));
        assert!(rendered.contains("high=1"));
    }

    fn task(
        id: &str,
        state: TaskState,
        lead_time_hours: i64,
        risk_level: Option<ImpactRiskLevel>,
    ) -> ExecutionTask {
        let created_at = Utc.with_ymd_and_hms(2026, 4, 10, 0, 0, 0).unwrap();
        let mut task = ExecutionTask::new(id, format!("Task {id}"));
        task.current_state = state;
        task.created_at = created_at;
        task.updated_at = created_at + Duration::hours(lead_time_hours);
        task.impact = risk_level.map(|risk_level| TaskImpact {
            risk_level,
            affected_symbols: 1,
            depth1: vec!["symbol".to_string()],
            analyzed_at: created_at,
            analyzed_commit: None,
            input_hash: None,
        });
        task
    }
}
