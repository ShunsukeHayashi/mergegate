use crate::store::{TaskState, TasksSnapshot};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;

fn state_str(state: TaskState) -> String {
    serde_json::to_value(state)
        .expect("failed to serialize TaskState to JSON value")
        .as_str()
        .expect("TaskState serialized to a non-string JSON value")
        .to_owned()
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReport {
    pub orphaned_locks: Vec<String>,
    pub invalid_transitions: Vec<String>,
    pub circular_dependencies: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationSeverity {
    Clean,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationJsonReport {
    pub severity: ValidationSeverity,
    pub exit_code: i32,
    pub issue_count: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub orphaned_locks: Vec<String>,
    pub invalid_transitions: Vec<String>,
    pub circular_dependencies: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn validate_snapshot(snapshot: &TasksSnapshot) -> ValidationReport {
    let mut report = ValidationReport::default();
    let task_map = snapshot
        .tasks
        .iter()
        .map(|task| (task.id.as_str(), task))
        .collect::<HashMap<_, _>>();

    for (file, entry) in &snapshot.file_locks {
        match task_map.get(entry.task_id.as_str()) {
            None => report.orphaned_locks.push(format!(
                "file lock '{file}' points to missing task '{}'",
                entry.task_id
            )),
            Some(task) => match &task.lock {
                None => report.orphaned_locks.push(format!(
                    "file lock '{file}' points to task '{}' without an active lock",
                    task.id
                )),
                Some(lock)
                    if !lock
                        .affected_files
                        .iter()
                        .any(|locked_file| locked_file == file) =>
                {
                    report.orphaned_locks.push(format!(
                        "file lock '{file}' is not tracked by task '{}'",
                        task.id
                    ));
                }
                Some(_) => {}
            },
        }
    }

    for task in &snapshot.tasks {
        if let Some(lock) = &task.lock {
            if task.current_state != TaskState::Implementing {
                report.invalid_transitions.push(format!(
                    "task '{}' is {} but still holds a lock",
                    task.id,
                    state_str(task.current_state)
                ));
            }

            for file in &lock.affected_files {
                match snapshot.file_locks.get(file) {
                    Some(entry) if entry.task_id == task.id => {}
                    Some(entry) => report.orphaned_locks.push(format!(
                        "task '{}' expects lock for '{}' but ownership is '{}'",
                        task.id, file, entry.task_id
                    )),
                    None => report.orphaned_locks.push(format!(
                        "task '{}' expects lock for '{}' but file_locks is missing it",
                        task.id, file
                    )),
                }
            }
        } else if task.current_state == TaskState::Implementing {
            report.invalid_transitions.push(format!(
                "task '{}' is implementing without an active lock",
                task.id
            ));
        }

        let unresolved_dependencies = task
            .dependencies
            .iter()
            .filter_map(|dependency| match task_map.get(dependency.as_str()) {
                Some(dep_task)
                    if matches!(dep_task.current_state, TaskState::Done | TaskState::Merged) =>
                {
                    None
                }
                Some(_) => Some(dependency.clone()),
                None => {
                    report.warnings.push(format!(
                        "task '{}' references missing dependency '{}'",
                        task.id, dependency
                    ));
                    None
                }
            })
            .collect::<Vec<_>>();

        if !unresolved_dependencies.is_empty()
            && matches!(
                task.current_state,
                TaskState::Analyzing
                    | TaskState::Implementing
                    | TaskState::Reviewing
                    | TaskState::AwaitingGithubSync
                    | TaskState::Merged
                    | TaskState::Deploying
                    | TaskState::Done
            )
        {
            report.invalid_transitions.push(format!(
                "task '{}' is {} with unresolved dependencies: {}",
                task.id,
                state_str(task.current_state),
                unresolved_dependencies.join(", ")
            ));
        }
    }

    report.circular_dependencies = detect_circular_dependencies(snapshot);
    report
}

impl ValidationReport {
    pub fn severity(&self) -> ValidationSeverity {
        if !self.orphaned_locks.is_empty()
            || !self.invalid_transitions.is_empty()
            || !self.circular_dependencies.is_empty()
        {
            ValidationSeverity::Error
        } else if !self.warnings.is_empty() {
            ValidationSeverity::Warning
        } else {
            ValidationSeverity::Clean
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self.severity() {
            ValidationSeverity::Clean => 0,
            ValidationSeverity::Warning => 1,
            ValidationSeverity::Error => 2,
        }
    }

    pub fn issue_count(&self) -> usize {
        self.orphaned_locks.len()
            + self.invalid_transitions.len()
            + self.circular_dependencies.len()
            + self.warnings.len()
    }

    pub fn error_count(&self) -> usize {
        self.orphaned_locks.len()
            + self.invalid_transitions.len()
            + self.circular_dependencies.len()
    }

    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }

    pub fn to_json_report(&self) -> ValidationJsonReport {
        ValidationJsonReport {
            severity: self.severity(),
            exit_code: self.exit_code(),
            issue_count: self.issue_count(),
            error_count: self.error_count(),
            warning_count: self.warning_count(),
            orphaned_locks: self.orphaned_locks.clone(),
            invalid_transitions: self.invalid_transitions.clone(),
            circular_dependencies: self.circular_dependencies.clone(),
            warnings: self.warnings.clone(),
        }
    }
}

impl fmt::Display for ValidationReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let severity = match self.severity() {
            ValidationSeverity::Clean => "clean",
            ValidationSeverity::Warning => "warning",
            ValidationSeverity::Error => "error",
        };

        if self.severity() == ValidationSeverity::Clean {
            return write!(f, "{severity} (0 issues)");
        }

        writeln!(f, "{severity} ({} issues)", self.issue_count())?;
        write_section(f, "orphaned_locks", &self.orphaned_locks)?;
        write_section(f, "invalid_transitions", &self.invalid_transitions)?;
        write_section(f, "circular_dependencies", &self.circular_dependencies)?;
        write_section(f, "warnings", &self.warnings)?;
        Ok(())
    }
}

fn write_section(f: &mut fmt::Formatter<'_>, name: &str, entries: &[String]) -> fmt::Result {
    writeln!(f, "- {name}: {}", entries.len())?;
    for entry in entries {
        writeln!(f, "  - {entry}")?;
    }
    Ok(())
}

fn detect_circular_dependencies(snapshot: &TasksSnapshot) -> Vec<String> {
    let graph = snapshot
        .tasks
        .iter()
        .map(|task| (task.id.as_str(), task.dependencies.as_slice()))
        .collect::<HashMap<_, _>>();
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    let mut cycles = HashSet::new();
    let mut path = Vec::new();

    for task in &snapshot.tasks {
        visit_dependency_path(
            task.id.as_str(),
            &graph,
            &mut visiting,
            &mut visited,
            &mut path,
            &mut cycles,
        );
    }

    let mut ordered = cycles.into_iter().collect::<Vec<_>>();
    ordered.sort();
    ordered
}

fn visit_dependency_path(
    task_id: &str,
    graph: &HashMap<&str, &[String]>,
    visiting: &mut HashSet<String>,
    visited: &mut HashSet<String>,
    path: &mut Vec<String>,
    cycles: &mut HashSet<String>,
) {
    if visited.contains(task_id) {
        return;
    }

    let inserted = visiting.insert(task_id.to_string());
    if inserted {
        path.push(task_id.to_string());
    }

    if let Some(dependencies) = graph.get(task_id) {
        for dependency in *dependencies {
            if let Some(cycle_start) = path.iter().position(|node| node == dependency) {
                let mut cycle = path[cycle_start..].to_vec();
                cycle.push(dependency.clone());
                cycles.insert(cycle.join(" -> "));
                continue;
            }

            if graph.contains_key(dependency.as_str()) {
                visit_dependency_path(dependency, graph, visiting, visited, path, cycles);
            }
        }
    }

    if inserted {
        visiting.remove(task_id);
        visited.insert(task_id.to_string());
        path.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::{validate_snapshot, ValidationReport, ValidationSeverity};
    use crate::store::{ExecutionTask, FileLockEntry, TaskLockSnapshot, TaskState, TasksSnapshot};
    use chrono::{Duration, Utc};
    use std::collections::HashMap;

    #[test]
    fn clean_snapshot() {
        let snapshot = TasksSnapshot {
            tasks: vec![
                task("task-a", TaskState::Done, vec![]),
                task("task-b", TaskState::Pending, vec!["task-a"]),
            ],
            ..TasksSnapshot::default()
        };

        let report = validate_snapshot(&snapshot);

        assert_eq!(report, ValidationReport::default());
    }

    #[test]
    fn orphaned_lock() {
        let mut snapshot = TasksSnapshot::default();
        snapshot.file_locks.insert(
            "src/lib.rs".into(),
            FileLockEntry {
                task_id: "missing-task".into(),
                agent: "codex".into(),
                node: "macbook".into(),
                expires_at: Utc::now() + Duration::seconds(60),
            },
        );

        let report = validate_snapshot(&snapshot);

        assert_eq!(report.orphaned_locks.len(), 1);
        assert!(report.orphaned_locks[0].contains("missing-task"));
    }

    #[test]
    fn invalid_transition() {
        let snapshot = TasksSnapshot {
            tasks: vec![task("task-a", TaskState::Implementing, vec![])],
            ..TasksSnapshot::default()
        };

        let report = validate_snapshot(&snapshot);

        assert_eq!(report.invalid_transitions.len(), 1);
        assert!(report.invalid_transitions[0].contains("without an active lock"));
    }

    #[test]
    fn invalid_transition_message_uses_serialised_state_name() {
        // State messages must use snake_case (serialised form), not {:?} Debug output.
        let snapshot = TasksSnapshot {
            tasks: vec![task("task-a", TaskState::Implementing, vec![])],
            ..TasksSnapshot::default()
        };

        let report = validate_snapshot(&snapshot);

        assert!(
            report.invalid_transitions[0].contains("implementing"),
            "expected 'implementing' (snake_case), got: {}",
            report.invalid_transitions[0]
        );
        assert!(
            !report.invalid_transitions[0].contains("Implementing"),
            "message must not use Debug (PascalCase) formatting"
        );
    }

    #[test]
    fn dependency_transition_message_uses_serialised_state_name() {
        // The "unresolved dependencies" path (line ~128) must also use snake_case.
        // task-b is Pending (unresolved), and task-a is Implementing which is an
        // active state → triggers the unresolved-dependencies invalid_transition message.
        let snapshot = TasksSnapshot {
            tasks: vec![
                task("task-a", TaskState::Implementing, vec!["task-b"]),
                task("task-b", TaskState::Pending, vec![]),
            ],
            ..TasksSnapshot::default()
        };

        let report = validate_snapshot(&snapshot);

        let dep_msg = report
            .invalid_transitions
            .iter()
            .find(|msg| msg.contains("unresolved dependencies"))
            .expect("expected an unresolved-dependencies message");

        assert!(
            dep_msg.contains("implementing"),
            "expected 'implementing' (snake_case), got: {dep_msg}"
        );
        assert!(
            !dep_msg.contains("Implementing"),
            "message must not use Debug (PascalCase) formatting"
        );
    }

    #[test]
    fn circular_dependency() {
        let snapshot = TasksSnapshot {
            tasks: vec![
                task("task-a", TaskState::Pending, vec!["task-b"]),
                task("task-b", TaskState::Pending, vec!["task-a"]),
            ],
            ..TasksSnapshot::default()
        };

        let report = validate_snapshot(&snapshot);

        assert_eq!(report.circular_dependencies.len(), 1);
        assert!(report.circular_dependencies[0].contains("task-a -> task-b -> task-a"));
    }

    #[test]
    fn multiple_issues() {
        let mut locked_task = task("task-a", TaskState::Reviewing, vec!["task-b"]);
        locked_task.lock = Some(task_lock(&["src/main.rs"]));

        let snapshot = TasksSnapshot {
            tasks: vec![locked_task, task("task-b", TaskState::Pending, vec![])],
            file_locks: HashMap::from([(
                "src/main.rs".into(),
                FileLockEntry {
                    task_id: "task-c".into(),
                    agent: "codex".into(),
                    node: "macbook".into(),
                    expires_at: Utc::now() + Duration::seconds(60),
                },
            )]),
            ..TasksSnapshot::default()
        };

        let report = validate_snapshot(&snapshot);

        assert!(!report.orphaned_locks.is_empty());
        assert!(!report.invalid_transitions.is_empty());
        assert!(report.circular_dependencies.is_empty());
    }

    #[test]
    fn empty_snapshot() {
        let snapshot = TasksSnapshot::default();

        let report = validate_snapshot(&snapshot);

        assert_eq!(report, ValidationReport::default());
        assert_eq!(report.to_string(), "clean (0 issues)");
    }

    #[test]
    fn warnings_only_are_reported() {
        let mut task = task("task-a", TaskState::Pending, vec!["missing"]);
        task.dependencies = vec!["missing".to_string()];
        let snapshot = TasksSnapshot {
            tasks: vec![task],
            ..TasksSnapshot::default()
        };

        let report = validate_snapshot(&snapshot);

        assert_eq!(report.severity(), ValidationSeverity::Warning);
        assert_eq!(report.exit_code(), 1);
        assert_eq!(report.warnings.len(), 1);
    }

    #[test]
    fn invalid_transitions_are_errors() {
        let snapshot = TasksSnapshot {
            tasks: vec![task("task-a", TaskState::Implementing, vec![])],
            ..TasksSnapshot::default()
        };

        let report = validate_snapshot(&snapshot);

        assert_eq!(report.severity(), ValidationSeverity::Error);
        assert_eq!(report.exit_code(), 2);
    }

    #[test]
    fn json_report_includes_fixed_counts() {
        let snapshot = TasksSnapshot {
            tasks: vec![task("task-a", TaskState::Implementing, vec![])],
            ..TasksSnapshot::default()
        };

        let report = validate_snapshot(&snapshot).to_json_report();

        assert_eq!(report.severity, ValidationSeverity::Error);
        assert_eq!(report.exit_code, 2);
        assert_eq!(report.issue_count, 1);
        assert_eq!(report.error_count, 1);
        assert_eq!(report.warning_count, 0);
    }

    fn task(id: &str, state: TaskState, dependencies: Vec<&str>) -> ExecutionTask {
        let mut task = ExecutionTask::new(id, id);
        task.current_state = state;
        task.dependencies = dependencies.into_iter().map(str::to_string).collect();
        task
    }

    fn task_lock(files: &[&str]) -> TaskLockSnapshot {
        let now = Utc::now();
        TaskLockSnapshot {
            locked_by: "codex@macbook".into(),
            locked_at: now,
            lease_duration_sec: 60,
            last_heartbeat: now,
            affected_files: files.iter().map(|file| (*file).to_string()).collect(),
        }
    }
}
