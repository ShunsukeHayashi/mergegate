//! DAG-based task graph for parallel execution
//!
//! This module provides a Directed Acyclic Graph (DAG) for organizing tasks
//! with dependencies, enabling parallel execution of independent tasks.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Unique identifier for a task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(Uuid);

impl TaskId {
    /// Create a new task ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create from UUID
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A task to be executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Task ID
    pub id: TaskId,
    /// Task name/description
    pub name: String,
    /// Task prompt (for agent execution)
    pub prompt: String,
    /// Estimated execution time in seconds
    pub estimated_time: Option<u64>,
    /// Priority (higher = more important)
    pub priority: u32,
}

impl Task {
    /// Create a new task
    pub fn new(name: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            id: TaskId::new(),
            name: name.into(),
            prompt: prompt.into(),
            estimated_time: None,
            priority: 0,
        }
    }

    /// Set estimated time
    pub fn with_estimated_time(mut self, seconds: u64) -> Self {
        self.estimated_time = Some(seconds);
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }
}

/// A node in the task graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskNode {
    /// Task ID
    pub id: TaskId,
    /// Task details
    pub task: Task,
    /// Dependencies (tasks that must complete before this one)
    pub dependencies: Vec<TaskId>,
    /// Dependents (tasks that depend on this one)
    pub dependents: Vec<TaskId>,
}

impl TaskNode {
    /// Create a new task node
    pub fn new(task: Task) -> Self {
        Self {
            id: task.id,
            task,
            dependencies: Vec::new(),
            dependents: Vec::new(),
        }
    }

    /// Add a dependency
    pub fn add_dependency(&mut self, dep_id: TaskId) {
        if !self.dependencies.contains(&dep_id) {
            self.dependencies.push(dep_id);
        }
    }

    /// Add a dependent
    pub fn add_dependent(&mut self, dependent_id: TaskId) {
        if !self.dependents.contains(&dependent_id) {
            self.dependents.push(dependent_id);
        }
    }

    /// Check if task has no dependencies
    pub fn is_root(&self) -> bool {
        self.dependencies.is_empty()
    }

    /// Get number of dependencies
    pub fn indegree(&self) -> usize {
        self.dependencies.len()
    }
}

/// A level in the task graph (tasks that can run in parallel)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskLevel {
    /// Level number (0 = root level, no dependencies)
    pub level: usize,
    /// Tasks at this level (can all run in parallel)
    pub tasks: Vec<Task>,
}

impl TaskLevel {
    /// Create a new task level
    pub fn new(level: usize) -> Self {
        Self {
            level,
            tasks: Vec::new(),
        }
    }

    /// Add a task to this level
    pub fn add_task(&mut self, task: Task) {
        self.tasks.push(task);
    }

    /// Get number of tasks at this level
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Check if level is empty
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}

/// Error types for DAG operations
#[derive(Debug, thiserror::Error)]
pub enum DAGError {
    #[error("Task not found: {0}")]
    TaskNotFound(String),
    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),
    #[error("Invalid graph: {0}")]
    InvalidGraph(String),
}

/// Directed Acyclic Graph of tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskGraph {
    /// All nodes in the graph
    nodes: HashMap<TaskId, TaskNode>,
    /// Task levels (for parallel execution)
    levels: Vec<TaskLevel>,
    /// Maximum parallelism allowed
    max_parallelism: usize,
}

impl TaskGraph {
    /// Create a new empty task graph
    pub fn new(max_parallelism: usize) -> Self {
        Self {
            nodes: HashMap::new(),
            levels: Vec::new(),
            max_parallelism,
        }
    }

    /// Add a task to the graph
    pub fn add_task(&mut self, task: Task) -> TaskId {
        let id = task.id;
        let node = TaskNode::new(task);
        self.nodes.insert(id, node);
        id
    }

    /// Add a dependency (from_task depends on to_task)
    pub fn add_dependency(&mut self, from_task: TaskId, to_task: TaskId) -> Result<(), DAGError> {
        // Check both nodes exist
        if !self.nodes.contains_key(&from_task) {
            return Err(DAGError::TaskNotFound(from_task.to_string()));
        }
        if !self.nodes.contains_key(&to_task) {
            return Err(DAGError::TaskNotFound(to_task.to_string()));
        }

        // Add dependency to from_task
        if let Some(node) = self.nodes.get_mut(&from_task) {
            node.add_dependency(to_task);
        }

        // Add dependent to to_task
        if let Some(node) = self.nodes.get_mut(&to_task) {
            node.add_dependent(from_task);
        }

        Ok(())
    }

    /// Get a task node by ID
    pub fn get_node(&self, id: &TaskId) -> Option<&TaskNode> {
        self.nodes.get(id)
    }

    /// Get task by ID
    pub fn get_task(&self, id: &TaskId) -> Option<&Task> {
        self.nodes.get(id).map(|n| &n.task)
    }

    /// Get all task IDs
    pub fn task_ids(&self) -> Vec<TaskId> {
        self.nodes.keys().copied().collect()
    }

    /// Find root nodes (nodes with no dependencies)
    pub fn find_roots(&self) -> Vec<TaskId> {
        self.nodes
            .values()
            .filter(|node| node.is_root())
            .map(|node| node.id)
            .collect()
    }

    /// Perform topological sort using Kahn's algorithm
    pub fn topological_sort(&self) -> Result<Vec<TaskId>, DAGError> {
        // Count in-degrees for all nodes
        let mut indegree: HashMap<TaskId, usize> = HashMap::new();
        for node in self.nodes.values() {
            indegree.insert(node.id, node.indegree());
        }

        // Start with nodes that have zero in-degree
        let mut queue: Vec<TaskId> = self
            .nodes
            .values()
            .filter(|node| node.indegree() == 0)
            .map(|node| node.id)
            .collect();

        let mut sorted = Vec::new();

        while let Some(task_id) = queue.pop() {
            sorted.push(task_id);

            if let Some(node) = self.nodes.get(&task_id) {
                for &dependent_id in &node.dependents {
                    if let Some(degree) = indegree.get_mut(&dependent_id) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push(dependent_id);
                        }
                    }
                }
            }
        }

        // If not all nodes were processed, there's a cycle
        if sorted.len() != self.nodes.len() {
            return Err(DAGError::CircularDependency(
                "Cycle detected in task graph".to_string(),
            ));
        }

        Ok(sorted)
    }

    /// Build task levels from topologically sorted tasks
    pub fn build_levels(&mut self) -> Result<(), DAGError> {
        let sorted = self.topological_sort()?;

        // Track which tasks have been assigned to levels
        let mut assigned: HashSet<TaskId> = HashSet::new();
        let mut current_level = 0;

        while assigned.len() < sorted.len() {
            let mut level = TaskLevel::new(current_level);

            // Find tasks whose dependencies are all assigned
            for &task_id in &sorted {
                if assigned.contains(&task_id) {
                    continue;
                }

                if let Some(node) = self.nodes.get(&task_id) {
                    let all_deps_assigned = node
                        .dependencies
                        .iter()
                        .all(|dep_id| assigned.contains(dep_id));

                    if all_deps_assigned {
                        level.add_task(node.task.clone());
                        assigned.insert(task_id);

                        // Respect max parallelism
                        if level.task_count() >= self.max_parallelism {
                            break;
                        }
                    }
                }
            }

            if level.is_empty() {
                return Err(DAGError::InvalidGraph(
                    "Unable to assign all tasks to levels".to_string(),
                ));
            }

            self.levels.push(level);
            current_level += 1;
        }

        Ok(())
    }

    /// Get task levels
    pub fn levels(&self) -> &[TaskLevel] {
        &self.levels
    }

    /// Get maximum parallelism
    pub fn max_parallelism(&self) -> usize {
        self.max_parallelism
    }

    /// Get total number of tasks
    pub fn task_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get total number of levels
    pub fn level_count(&self) -> usize {
        self.levels.len()
    }

    /// Check if graph is empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

/// Builder for creating task graphs with a fluent API
pub struct TaskGraphBuilder {
    graph: TaskGraph,
    task_names: HashMap<String, TaskId>,
}

impl TaskGraphBuilder {
    /// Create a new builder
    pub fn new(max_parallelism: usize) -> Self {
        Self {
            graph: TaskGraph::new(max_parallelism),
            task_names: HashMap::new(),
        }
    }

    /// Add a task
    pub fn add_task(mut self, name: impl Into<String>, prompt: impl Into<String>) -> Self {
        let name = name.into();
        let task = Task::new(name.clone(), prompt);
        let id = self.graph.add_task(task);
        self.task_names.insert(name, id);
        self
    }

    /// Add a task with priority
    pub fn add_task_with_priority(
        mut self,
        name: impl Into<String>,
        prompt: impl Into<String>,
        priority: u32,
    ) -> Self {
        let name = name.into();
        let task = Task::new(name.clone(), prompt).with_priority(priority);
        let id = self.graph.add_task(task);
        self.task_names.insert(name, id);
        self
    }

    /// Add a dependency between tasks by name
    pub fn add_dependency(
        mut self,
        from_task: impl Into<String>,
        to_task: impl Into<String>,
    ) -> Result<Self, DAGError> {
        let from_name = from_task.into();
        let to_name = to_task.into();

        let from_id = self
            .task_names
            .get(&from_name)
            .copied()
            .ok_or(DAGError::TaskNotFound(from_name))?;
        let to_id = self
            .task_names
            .get(&to_name)
            .copied()
            .ok_or(DAGError::TaskNotFound(to_name))?;

        self.graph.add_dependency(from_id, to_id)?;
        Ok(self)
    }

    /// Build and return the task graph
    pub fn build(mut self) -> Result<TaskGraph, DAGError> {
        self.graph.build_levels()?;
        Ok(self.graph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_graph() {
        let graph = TaskGraph::new(4);
        assert!(graph.is_empty());
        assert_eq!(graph.task_count(), 0);
    }

    #[test]
    fn test_add_task() {
        let mut graph = TaskGraph::new(4);
        let task = Task::new("test", "Test prompt");
        let id = graph.add_task(task);

        assert_eq!(graph.task_count(), 1);
        assert!(graph.get_task(&id).is_some());
    }

    #[test]
    fn test_topological_sort() {
        let mut graph = TaskGraph::new(4);

        // Create linear dependency: task1 -> task2 -> task3
        let task1 = Task::new("task1", "First task");
        let task2 = Task::new("task2", "Second task");
        let task3 = Task::new("task3", "Third task");

        let id1 = graph.add_task(task1);
        let id2 = graph.add_task(task2);
        let id3 = graph.add_task(task3);

        graph.add_dependency(id2, id1).unwrap();
        graph.add_dependency(id3, id2).unwrap();

        let sorted = graph.topological_sort().unwrap();
        assert_eq!(sorted.len(), 3);

        let pos1 = sorted.iter().position(|&id| id == id1).unwrap();
        let pos2 = sorted.iter().position(|&id| id == id2).unwrap();
        let pos3 = sorted.iter().position(|&id| id == id3).unwrap();

        assert!(pos1 < pos2);
        assert!(pos2 < pos3);
    }

    #[test]
    fn test_build_levels() {
        let mut graph = TaskGraph::new(2);

        // Diamond pattern
        let task1 = Task::new("task1", "Root");
        let task2 = Task::new("task2", "Left");
        let task3 = Task::new("task3", "Right");
        let task4 = Task::new("task4", "Bottom");

        let id1 = graph.add_task(task1);
        let id2 = graph.add_task(task2);
        let id3 = graph.add_task(task3);
        let id4 = graph.add_task(task4);

        graph.add_dependency(id2, id1).unwrap();
        graph.add_dependency(id3, id1).unwrap();
        graph.add_dependency(id4, id2).unwrap();
        graph.add_dependency(id4, id3).unwrap();

        graph.build_levels().unwrap();

        assert!(graph.level_count() >= 2);
        assert!(graph.levels()[0].task_count() >= 1);
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = TaskGraph::new(4);

        let task1 = Task::new("task1", "First");
        let task2 = Task::new("task2", "Second");

        let id1 = graph.add_task(task1);
        let id2 = graph.add_task(task2);

        graph.add_dependency(id1, id2).unwrap();
        graph.add_dependency(id2, id1).unwrap(); // Create cycle

        let result = graph.topological_sort();
        assert!(result.is_err());
    }

    #[test]
    fn test_builder() {
        let graph = TaskGraphBuilder::new(4)
            .add_task("setup", "Setup environment")
            .add_task("build", "Build project")
            .add_task("test", "Run tests")
            .add_dependency("build", "setup")
            .unwrap()
            .add_dependency("test", "build")
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(graph.task_count(), 3);
        assert!(graph.level_count() >= 1);
    }

    #[test]
    fn test_find_roots() {
        let mut graph = TaskGraph::new(4);

        let task1 = Task::new("root1", "First root");
        let task2 = Task::new("root2", "Second root");
        let task3 = Task::new("child", "Child task");

        let id1 = graph.add_task(task1);
        let id2 = graph.add_task(task2);
        let id3 = graph.add_task(task3);

        graph.add_dependency(id3, id1).unwrap();
        graph.add_dependency(id3, id2).unwrap();

        let roots = graph.find_roots();
        assert_eq!(roots.len(), 2);
        assert!(roots.contains(&id1));
        assert!(roots.contains(&id2));
    }

    #[test]
    fn test_task_id_default() {
        let id = TaskId::default();
        assert!(!id.to_string().is_empty());
    }

    #[test]
    fn test_task_with_priority() {
        let task = Task::new("priority-task", "Task with priority")
            .with_priority(10);

        assert_eq!(task.priority, 10);
        assert_eq!(task.name, "priority-task");
    }

    #[test]
    fn test_task_with_estimated_time() {
        let task = Task::new("timed-task", "Task with time estimate")
            .with_estimated_time(120);

        assert_eq!(task.estimated_time, Some(120));
    }

    #[test]
    fn test_task_node_is_root() {
        let task = Task::new("root-task", "Root task");
        let node = TaskNode::new(task);

        assert!(node.is_root());
        assert_eq!(node.indegree(), 0);
    }

    #[test]
    fn test_task_node_add_dependency() {
        let task1 = Task::new("task1", "First task");
        let task2 = Task::new("task2", "Second task");
        let mut node = TaskNode::new(task1);
        let dep_id = task2.id;

        node.add_dependency(dep_id);
        assert!(!node.is_root());
        assert_eq!(node.indegree(), 1);
        assert!(node.dependencies.contains(&dep_id));
    }

    #[test]
    fn test_task_node_add_dependent() {
        let task1 = Task::new("task1", "First task");
        let task2 = Task::new("task2", "Second task");
        let mut node = TaskNode::new(task1);
        let dependent_id = task2.id;

        node.add_dependent(dependent_id);
        assert!(node.dependents.contains(&dependent_id));
    }

    #[test]
    fn test_task_node_no_duplicate_dependencies() {
        let task1 = Task::new("task1", "First task");
        let task2 = Task::new("task2", "Second task");
        let mut node = TaskNode::new(task1);
        let dep_id = task2.id;

        node.add_dependency(dep_id);
        node.add_dependency(dep_id); // Add again

        assert_eq!(node.dependencies.len(), 1);
    }

    #[test]
    fn test_task_level_creation() {
        let level = TaskLevel::new(0);
        assert_eq!(level.level, 0);
        assert!(level.is_empty());
        assert_eq!(level.task_count(), 0);
    }

    #[test]
    fn test_task_level_add_task() {
        let mut level = TaskLevel::new(1);
        let task = Task::new("task1", "First task");

        level.add_task(task);
        assert!(!level.is_empty());
        assert_eq!(level.task_count(), 1);
    }

    #[test]
    fn test_graph_task_count() {
        let mut graph = TaskGraph::new(4);
        assert_eq!(graph.task_count(), 0);

        let task = Task::new("task", "Test task");
        graph.add_task(task);
        assert_eq!(graph.task_count(), 1);
    }

    #[test]
    fn test_graph_max_parallelism() {
        let graph = TaskGraph::new(8);
        assert_eq!(graph.max_parallelism(), 8);
    }

    #[test]
    fn test_graph_get_node() {
        let mut graph = TaskGraph::new(4);
        let task = Task::new("test", "Test task");
        let id = graph.add_task(task);

        let node = graph.get_node(&id);
        assert!(node.is_some());
        assert_eq!(node.unwrap().task.name, "test");
    }

    #[test]
    fn test_graph_task_ids() {
        let mut graph = TaskGraph::new(4);
        let task1 = Task::new("task1", "First");
        let task2 = Task::new("task2", "Second");

        let id1 = graph.add_task(task1);
        let id2 = graph.add_task(task2);

        let ids = graph.task_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
    }

    #[test]
    fn test_graph_levels() {
        let mut graph = TaskGraph::new(4);

        let task1 = Task::new("task1", "First");
        let task2 = Task::new("task2", "Second");

        let id1 = graph.add_task(task1);
        let id2 = graph.add_task(task2);

        graph.add_dependency(id2, id1).unwrap();
        graph.build_levels().unwrap();

        let levels = graph.levels();
        assert!(!levels.is_empty());
        // Tasks are added to levels as their dependencies are satisfied
        assert!(graph.level_count() >= 1);
    }

    #[test]
    fn test_builder_with_priority() {
        let graph = TaskGraphBuilder::new(4)
            .add_task_with_priority("high-priority", "Important task", 100)
            .build()
            .unwrap();

        assert_eq!(graph.task_count(), 1);
    }

    #[test]
    fn test_dag_error_task_not_found() {
        let error = DAGError::TaskNotFound("missing-task".to_string());
        let display = format!("{}", error);
        assert!(display.contains("missing-task"));
    }

    #[test]
    fn test_dag_error_circular_dependency() {
        let error = DAGError::CircularDependency("cycle detected".to_string());
        let display = format!("{}", error);
        assert!(display.contains("cycle"));
    }

    #[test]
    fn test_dag_error_invalid_graph() {
        let error = DAGError::InvalidGraph("invalid".to_string());
        let display = format!("{}", error);
        assert!(display.contains("invalid"));
    }

    #[test]
    fn test_complex_parallel_graph() {
        // Create a graph with multiple parallel paths
        // Root -> [A, B, C] -> Merge
        let mut graph = TaskGraph::new(4);

        let root = Task::new("root", "Root task");
        let a = Task::new("a", "Task A");
        let b = Task::new("b", "Task B");
        let c = Task::new("c", "Task C");
        let merge = Task::new("merge", "Merge task");

        let root_id = graph.add_task(root);
        let a_id = graph.add_task(a);
        let b_id = graph.add_task(b);
        let c_id = graph.add_task(c);
        let merge_id = graph.add_task(merge);

        graph.add_dependency(a_id, root_id).unwrap();
        graph.add_dependency(b_id, root_id).unwrap();
        graph.add_dependency(c_id, root_id).unwrap();
        graph.add_dependency(merge_id, a_id).unwrap();
        graph.add_dependency(merge_id, b_id).unwrap();
        graph.add_dependency(merge_id, c_id).unwrap();

        graph.build_levels().unwrap();

        // All 5 tasks should be assigned to levels
        assert!(graph.level_count() >= 1);
        let total_tasks: usize = graph.levels().iter().map(|l| l.task_count()).sum();
        assert_eq!(total_tasks, 5);
    }

    #[test]
    fn test_builder_dependency_error() {
        let result = TaskGraphBuilder::new(4)
            .add_task("task1", "First")
            .add_dependency("task1", "nonexistent");

        assert!(result.is_err());
    }

    #[test]
    fn test_single_task_graph() {
        let mut graph = TaskGraph::new(4);
        let task = Task::new("single", "Single task");
        graph.add_task(task);

        graph.build_levels().unwrap();

        assert_eq!(graph.level_count(), 1);
        assert_eq!(graph.levels()[0].task_count(), 1);
    }

    #[test]
    fn test_task_id_display() {
        let id = TaskId::new();
        let display = id.to_string();

        // Should be a valid UUID string
        assert!(!display.is_empty());
        assert!(display.contains('-')); // UUIDs contain hyphens
    }

    #[test]
    fn test_independent_tasks_parallel() {
        // All independent tasks should be in the same level
        let mut graph = TaskGraph::new(10);

        for i in 0..5 {
            let task = Task::new(format!("task{}", i), format!("Task {}", i));
            graph.add_task(task);
        }

        graph.build_levels().unwrap();

        // All tasks are independent, so they should be in level 0
        // (with max_parallelism=10, they can all run in parallel)
        assert_eq!(graph.level_count(), 1);
        assert_eq!(graph.levels()[0].task_count(), 5);
    }
}
