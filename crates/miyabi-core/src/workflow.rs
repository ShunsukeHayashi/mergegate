//! Multi-Agent Workflow System
//!
//! Orchestrate multiple agents in sequence or parallel execution.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::agent::{Agent, ExecutorRegistry};
use crate::AnthropicClient;

/// Workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// Workflow name
    pub name: String,
    /// Description
    #[serde(default)]
    pub description: String,
    /// Workflow steps
    pub steps: Vec<WorkflowStep>,
    /// Global variables
    #[serde(default)]
    pub variables: HashMap<String, String>,
    /// On failure behavior
    #[serde(default)]
    pub on_failure: FailurePolicy,
}

/// A single step in the workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    /// Step ID for reference
    pub id: String,
    /// Step name
    pub name: String,
    /// Agent spec to use
    #[serde(default)]
    pub agent: Option<String>,
    /// Task/prompt for the agent
    pub task: String,
    /// Dependencies (step IDs that must complete first)
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Condition for execution
    #[serde(default)]
    pub condition: Option<StepCondition>,
    /// Timeout in seconds
    #[serde(default = "default_step_timeout")]
    pub timeout: u64,
    /// Retry configuration
    #[serde(default)]
    pub retry: RetryConfig,
    /// Output variable name to store result
    #[serde(default)]
    pub output: Option<String>,
}

fn default_step_timeout() -> u64 {
    300
}

/// Condition for step execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[derive(Default)]
pub enum StepCondition {
    /// Always execute
    #[default]
    Always,
    /// Execute if previous step succeeded
    OnSuccess,
    /// Execute if previous step failed
    OnFailure,
    /// Execute if expression is true
    If { expression: String },
}

/// Retry configuration for a step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum retry attempts
    #[serde(default = "default_max_retries")]
    pub max_attempts: u32,
    /// Delay between retries in seconds
    #[serde(default = "default_retry_delay")]
    pub delay: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 1,
            delay: 5,
        }
    }
}

fn default_max_retries() -> u32 {
    1
}

fn default_retry_delay() -> u64 {
    5
}

/// Failure policy for workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum FailurePolicy {
    /// Stop workflow on first failure
    #[default]
    Stop,
    /// Continue with remaining steps
    Continue,
    /// Run cleanup steps
    Cleanup,
}

/// Step execution result
#[derive(Debug, Clone)]
pub struct StepResult {
    pub step_id: String,
    pub status: StepStatus,
    pub output: Option<String>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

/// Step execution status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

/// Workflow execution context
#[derive(Debug, Clone, Default)]
pub struct WorkflowContext {
    /// Variables available to steps
    pub variables: HashMap<String, String>,
    /// Results from completed steps
    pub results: HashMap<String, StepResult>,
}

impl WorkflowContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_variables(mut self, vars: HashMap<String, String>) -> Self {
        self.variables = vars;
        self
    }

    pub fn set_variable(&mut self, key: &str, value: &str) {
        self.variables.insert(key.to_string(), value.to_string());
    }

    pub fn get_variable(&self, key: &str) -> Option<&String> {
        self.variables.get(key)
    }

    pub fn add_result(&mut self, result: StepResult) {
        self.results.insert(result.step_id.clone(), result);
    }

    pub fn get_result(&self, step_id: &str) -> Option<&StepResult> {
        self.results.get(step_id)
    }

    /// Expand variables in a string
    pub fn expand(&self, input: &str) -> String {
        let mut result = input.to_string();
        for (key, value) in &self.variables {
            result = result.replace(&format!("${{{}}}", key), value);
        }
        result
    }
}

/// Workflow execution result
#[derive(Debug)]
pub struct WorkflowResult {
    pub workflow_name: String,
    pub status: WorkflowStatus,
    pub steps: Vec<StepResult>,
    pub duration_ms: u64,
}

/// Workflow execution status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowStatus {
    Completed,
    Failed,
    PartiallyCompleted,
}

/// Workflow manager for loading and executing workflows
pub struct WorkflowManager {
    workflows: HashMap<String, Workflow>,
}

impl Default for WorkflowManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkflowManager {
    /// Create a new workflow manager
    pub fn new() -> Self {
        Self {
            workflows: HashMap::new(),
        }
    }

    /// Load a workflow from a YAML file
    pub fn load_workflow(&mut self, path: &PathBuf) -> Result<String> {
        let content = std::fs::read_to_string(path)?;
        let workflow: Workflow = serde_yaml::from_str(&content)?;
        let name = workflow.name.clone();
        self.workflows.insert(name.clone(), workflow);
        Ok(name)
    }

    /// Load all workflows from a directory
    pub fn load_from_directory(&mut self, dir: &PathBuf) -> Result<Vec<String>> {
        let mut loaded = Vec::new();
        if dir.exists() && dir.is_dir() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path
                    .extension()
                    .map(|e| e == "yml" || e == "yaml")
                    .unwrap_or(false)
                {
                    if let Ok(name) = self.load_workflow(&path) {
                        loaded.push(name);
                    }
                }
            }
        }
        Ok(loaded)
    }

    /// Register a workflow directly
    pub fn register(&mut self, workflow: Workflow) {
        self.workflows.insert(workflow.name.clone(), workflow);
    }

    /// Get a workflow by name
    pub fn get_workflow(&self, name: &str) -> Option<&Workflow> {
        self.workflows.get(name)
    }

    /// List all registered workflows
    pub fn list_workflows(&self) -> Vec<&str> {
        self.workflows.keys().map(|s| s.as_str()).collect()
    }

    /// Execute a workflow
    pub async fn execute(
        &self,
        name: &str,
        initial_vars: HashMap<String, String>,
    ) -> Result<WorkflowResult> {
        let workflow = self
            .workflows
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Workflow not found: {}", name))?;

        let start_time = std::time::Instant::now();
        let mut context = WorkflowContext::new().with_variables(workflow.variables.clone());

        // Add initial variables
        for (key, value) in initial_vars {
            context.set_variable(&key, &value);
        }

        let mut step_results = Vec::new();
        let mut all_succeeded = true;

        // Build dependency graph and execute
        let execution_order = self.build_execution_order(workflow)?;

        for step_id in execution_order {
            let step = workflow
                .steps
                .iter()
                .find(|s| s.id == step_id)
                .ok_or_else(|| anyhow::anyhow!("Step not found: {}", step_id))?;

            // Check dependencies
            let deps_satisfied = step.depends_on.iter().all(|dep_id| {
                context
                    .get_result(dep_id)
                    .map(|r| r.status == StepStatus::Completed)
                    .unwrap_or(false)
            });

            if !deps_satisfied {
                let result = StepResult {
                    step_id: step.id.clone(),
                    status: StepStatus::Skipped,
                    output: None,
                    error: Some("Dependencies not satisfied".to_string()),
                    duration_ms: 0,
                };
                step_results.push(result.clone());
                context.add_result(result);
                continue;
            }

            // Check condition
            if !self.evaluate_condition(&step.condition, &context) {
                let result = StepResult {
                    step_id: step.id.clone(),
                    status: StepStatus::Skipped,
                    output: None,
                    error: Some("Condition not met".to_string()),
                    duration_ms: 0,
                };
                step_results.push(result.clone());
                context.add_result(result);
                continue;
            }

            // Execute step
            let step_start = std::time::Instant::now();
            let expanded_task = context.expand(&step.task);

            // Placeholder for actual agent execution
            let result = StepResult {
                step_id: step.id.clone(),
                status: StepStatus::Completed,
                output: Some(format!("Executed: {}", expanded_task)),
                error: None,
                duration_ms: step_start.elapsed().as_millis() as u64,
            };

            // Store output variable if specified
            if let Some(output_var) = &step.output {
                if let Some(output) = &result.output {
                    context.set_variable(output_var, output);
                }
            }

            if result.status == StepStatus::Failed {
                all_succeeded = false;
                if matches!(workflow.on_failure, FailurePolicy::Stop) {
                    step_results.push(result.clone());
                    context.add_result(result);
                    break;
                }
            }

            step_results.push(result.clone());
            context.add_result(result);
        }

        let status = if all_succeeded {
            WorkflowStatus::Completed
        } else if step_results
            .iter()
            .any(|r| r.status == StepStatus::Completed)
        {
            WorkflowStatus::PartiallyCompleted
        } else {
            WorkflowStatus::Failed
        };

        Ok(WorkflowResult {
            workflow_name: name.to_string(),
            status,
            steps: step_results,
            duration_ms: start_time.elapsed().as_millis() as u64,
        })
    }

    /// Execute a workflow with actual Agent execution
    pub async fn execute_with_agent(
        &self,
        name: &str,
        initial_vars: HashMap<String, String>,
        client: AnthropicClient,
        registry: ExecutorRegistry,
    ) -> Result<WorkflowResult> {
        let workflow = self
            .workflows
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Workflow not found: {}", name))?;

        let start_time = std::time::Instant::now();
        let mut context = WorkflowContext::new().with_variables(workflow.variables.clone());

        // Add initial variables
        for (key, value) in initial_vars {
            context.set_variable(&key, &value);
        }

        let mut step_results = Vec::new();
        let mut all_succeeded = true;

        // Build dependency graph and execute
        let execution_order = self.build_execution_order(workflow)?;

        for step_id in execution_order {
            let step = workflow
                .steps
                .iter()
                .find(|s| s.id == step_id)
                .ok_or_else(|| anyhow::anyhow!("Step not found: {}", step_id))?;

            // Check dependencies
            let deps_satisfied = step.depends_on.iter().all(|dep_id| {
                context
                    .get_result(dep_id)
                    .map(|r| r.status == StepStatus::Completed)
                    .unwrap_or(false)
            });

            if !deps_satisfied {
                let result = StepResult {
                    step_id: step.id.clone(),
                    status: StepStatus::Skipped,
                    output: None,
                    error: Some("Dependencies not satisfied".to_string()),
                    duration_ms: 0,
                };
                step_results.push(result.clone());
                context.add_result(result);
                continue;
            }

            // Check condition
            if !self.evaluate_condition(&step.condition, &context) {
                let result = StepResult {
                    step_id: step.id.clone(),
                    status: StepStatus::Skipped,
                    output: None,
                    error: Some("Condition not met".to_string()),
                    duration_ms: 0,
                };
                step_results.push(result.clone());
                context.add_result(result);
                continue;
            }

            // Execute step with Agent
            let step_start = std::time::Instant::now();
            let expanded_task = context.expand(&step.task);

            // Create agent for this step
            let agent = Agent::new(client.clone(), registry.clone());

            // Execute the task
            let result = match agent.run(&expanded_task).await {
                Ok(agent_result) => StepResult {
                    step_id: step.id.clone(),
                    status: StepStatus::Completed,
                    output: Some(agent_result.output),
                    error: None,
                    duration_ms: step_start.elapsed().as_millis() as u64,
                },
                Err(e) => StepResult {
                    step_id: step.id.clone(),
                    status: StepStatus::Failed,
                    output: None,
                    error: Some(e.to_string()),
                    duration_ms: step_start.elapsed().as_millis() as u64,
                },
            };

            // Store output variable if specified
            if let Some(output_var) = &step.output {
                if let Some(output) = &result.output {
                    context.set_variable(output_var, output);
                }
            }

            if result.status == StepStatus::Failed {
                all_succeeded = false;
                if matches!(workflow.on_failure, FailurePolicy::Stop) {
                    step_results.push(result.clone());
                    context.add_result(result);
                    break;
                }
            }

            step_results.push(result.clone());
            context.add_result(result);
        }

        let status = if all_succeeded {
            WorkflowStatus::Completed
        } else if step_results
            .iter()
            .any(|r| r.status == StepStatus::Completed)
        {
            WorkflowStatus::PartiallyCompleted
        } else {
            WorkflowStatus::Failed
        };

        Ok(WorkflowResult {
            workflow_name: name.to_string(),
            status,
            steps: step_results,
            duration_ms: start_time.elapsed().as_millis() as u64,
        })
    }

    /// Build execution order from dependencies (topological sort)
    fn build_execution_order(&self, workflow: &Workflow) -> Result<Vec<String>> {
        let mut order = Vec::new();
        let mut visited = HashMap::new();
        let mut temp_visited = HashMap::new();

        for step in &workflow.steps {
            if !visited.contains_key(&step.id) {
                self.visit_step(
                    workflow,
                    &step.id,
                    &mut visited,
                    &mut temp_visited,
                    &mut order,
                )?;
            }
        }

        Ok(order)
    }

    #[allow(clippy::only_used_in_recursion)]
    fn visit_step(
        &self,
        workflow: &Workflow,
        step_id: &str,
        visited: &mut HashMap<String, bool>,
        temp_visited: &mut HashMap<String, bool>,
        order: &mut Vec<String>,
    ) -> Result<()> {
        if temp_visited.get(step_id).copied().unwrap_or(false) {
            return Err(anyhow::anyhow!(
                "Circular dependency detected at step: {}",
                step_id
            ));
        }

        if visited.get(step_id).copied().unwrap_or(false) {
            return Ok(());
        }

        temp_visited.insert(step_id.to_string(), true);

        let step = workflow
            .steps
            .iter()
            .find(|s| s.id == step_id)
            .ok_or_else(|| anyhow::anyhow!("Step not found: {}", step_id))?;

        for dep_id in &step.depends_on {
            self.visit_step(workflow, dep_id, visited, temp_visited, order)?;
        }

        temp_visited.insert(step_id.to_string(), false);
        visited.insert(step_id.to_string(), true);
        order.push(step_id.to_string());

        Ok(())
    }

    /// Evaluate a step condition
    fn evaluate_condition(
        &self,
        condition: &Option<StepCondition>,
        context: &WorkflowContext,
    ) -> bool {
        match condition {
            None | Some(StepCondition::Always) => true,
            Some(StepCondition::OnSuccess) => context
                .results
                .values()
                .last()
                .map(|r| r.status == StepStatus::Completed)
                .unwrap_or(true),
            Some(StepCondition::OnFailure) => context
                .results
                .values()
                .last()
                .map(|r| r.status == StepStatus::Failed)
                .unwrap_or(false),
            Some(StepCondition::If { expression }) => {
                // Simple expression evaluation (variable == value)
                let expanded = context.expand(expression);
                !expanded.is_empty() && expanded != "false" && expanded != "0"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_context() {
        let mut context = WorkflowContext::new();
        context.set_variable("name", "test");
        context.set_variable("version", "1.0");

        assert_eq!(context.get_variable("name"), Some(&"test".to_string()));
        assert_eq!(
            context.expand("Project: ${name} v${version}"),
            "Project: test v1.0"
        );
    }

    #[test]
    fn test_workflow_serialization() {
        let workflow = Workflow {
            name: "test-workflow".to_string(),
            description: "Test".to_string(),
            steps: vec![WorkflowStep {
                id: "step1".to_string(),
                name: "First Step".to_string(),
                agent: Some("code-reviewer".to_string()),
                task: "Review code".to_string(),
                depends_on: vec![],
                condition: None,
                timeout: 300,
                retry: RetryConfig::default(),
                output: Some("review_result".to_string()),
            }],
            variables: HashMap::new(),
            on_failure: FailurePolicy::Stop,
        };

        let yaml = serde_yaml::to_string(&workflow).unwrap();
        assert!(yaml.contains("test-workflow"));
        assert!(yaml.contains("step1"));
    }

    #[test]
    fn test_workflow_manager() {
        let mut manager = WorkflowManager::new();
        let workflow = Workflow {
            name: "test".to_string(),
            description: "Test workflow".to_string(),
            steps: vec![],
            variables: HashMap::new(),
            on_failure: FailurePolicy::Stop,
        };

        manager.register(workflow);
        assert!(manager.get_workflow("test").is_some());
        assert_eq!(manager.list_workflows(), vec!["test"]);
    }

    #[tokio::test]
    async fn test_workflow_execution() {
        let mut manager = WorkflowManager::new();
        let workflow = Workflow {
            name: "simple".to_string(),
            description: "Simple workflow".to_string(),
            steps: vec![
                WorkflowStep {
                    id: "step1".to_string(),
                    name: "Step 1".to_string(),
                    agent: None,
                    task: "Task 1".to_string(),
                    depends_on: vec![],
                    condition: None,
                    timeout: 30,
                    retry: RetryConfig::default(),
                    output: None,
                },
                WorkflowStep {
                    id: "step2".to_string(),
                    name: "Step 2".to_string(),
                    agent: None,
                    task: "Task 2 depends on ${result}".to_string(),
                    depends_on: vec!["step1".to_string()],
                    condition: None,
                    timeout: 30,
                    retry: RetryConfig::default(),
                    output: None,
                },
            ],
            variables: HashMap::new(),
            on_failure: FailurePolicy::Stop,
        };

        manager.register(workflow);
        let result = manager.execute("simple", HashMap::new()).await.unwrap();

        assert_eq!(result.workflow_name, "simple");
        assert_eq!(result.status, WorkflowStatus::Completed);
        assert_eq!(result.steps.len(), 2);
    }

    #[test]
    fn test_step_condition_default() {
        let condition = StepCondition::default();
        assert!(matches!(condition, StepCondition::Always));
    }

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 1);
        assert_eq!(config.delay, 5);
    }

    #[test]
    fn test_failure_policy_default() {
        let policy = FailurePolicy::default();
        assert!(matches!(policy, FailurePolicy::Stop));
    }

    #[test]
    fn test_step_status_equality() {
        assert_eq!(StepStatus::Pending, StepStatus::Pending);
        assert_eq!(StepStatus::Completed, StepStatus::Completed);
        assert_ne!(StepStatus::Pending, StepStatus::Completed);
        assert_ne!(StepStatus::Failed, StepStatus::Skipped);
    }

    #[test]
    fn test_workflow_status_equality() {
        assert_eq!(WorkflowStatus::Completed, WorkflowStatus::Completed);
        assert_ne!(WorkflowStatus::Completed, WorkflowStatus::Failed);
    }

    #[test]
    fn test_workflow_context_with_variables() {
        let mut vars = HashMap::new();
        vars.insert("key1".to_string(), "value1".to_string());
        vars.insert("key2".to_string(), "value2".to_string());

        let context = WorkflowContext::new().with_variables(vars);
        assert_eq!(context.get_variable("key1"), Some(&"value1".to_string()));
        assert_eq!(context.get_variable("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_workflow_context_add_result() {
        let mut context = WorkflowContext::new();
        let result = StepResult {
            step_id: "test-step".to_string(),
            status: StepStatus::Completed,
            output: Some("output".to_string()),
            error: None,
            duration_ms: 100,
        };

        context.add_result(result);
        let retrieved = context.get_result("test-step");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().status, StepStatus::Completed);
    }

    #[test]
    fn test_step_result_creation() {
        let result = StepResult {
            step_id: "test-step".to_string(),
            status: StepStatus::Completed,
            output: Some("test output".to_string()),
            error: None,
            duration_ms: 150,
        };

        assert_eq!(result.step_id, "test-step");
        assert_eq!(result.status, StepStatus::Completed);
        assert_eq!(result.output, Some("test output".to_string()));
        assert!(result.error.is_none());
        assert_eq!(result.duration_ms, 150);
    }

    #[test]
    fn test_workflow_result_creation() {
        let result = WorkflowResult {
            workflow_name: "test-workflow".to_string(),
            status: WorkflowStatus::Completed,
            steps: vec![],
            duration_ms: 1000,
        };

        assert_eq!(result.workflow_name, "test-workflow");
        assert_eq!(result.status, WorkflowStatus::Completed);
        assert!(result.steps.is_empty());
        assert_eq!(result.duration_ms, 1000);
    }

    #[tokio::test]
    async fn test_workflow_with_output_variables() {
        let mut manager = WorkflowManager::new();
        let workflow = Workflow {
            name: "output-test".to_string(),
            description: "Test output variables".to_string(),
            steps: vec![
                WorkflowStep {
                    id: "step1".to_string(),
                    name: "Step 1".to_string(),
                    agent: None,
                    task: "First task".to_string(),
                    depends_on: vec![],
                    condition: None,
                    timeout: 30,
                    retry: RetryConfig::default(),
                    output: Some("first_result".to_string()),
                },
                WorkflowStep {
                    id: "step2".to_string(),
                    name: "Step 2".to_string(),
                    agent: None,
                    task: "Use ${first_result}".to_string(),
                    depends_on: vec!["step1".to_string()],
                    condition: None,
                    timeout: 30,
                    retry: RetryConfig::default(),
                    output: None,
                },
            ],
            variables: HashMap::new(),
            on_failure: FailurePolicy::Stop,
        };

        manager.register(workflow);
        let result = manager
            .execute("output-test", HashMap::new())
            .await
            .unwrap();

        assert_eq!(result.status, WorkflowStatus::Completed);
        assert_eq!(result.steps.len(), 2);
    }

    #[test]
    fn test_workflow_with_global_variables() {
        let mut vars = HashMap::new();
        vars.insert("project".to_string(), "miyabi".to_string());

        let workflow = Workflow {
            name: "vars-test".to_string(),
            description: "Test global variables".to_string(),
            steps: vec![],
            variables: vars,
            on_failure: FailurePolicy::Stop,
        };

        assert_eq!(
            workflow.variables.get("project"),
            Some(&"miyabi".to_string())
        );
    }

    #[test]
    fn test_workflow_step_with_all_fields() {
        let step = WorkflowStep {
            id: "complete-step".to_string(),
            name: "Complete Step".to_string(),
            agent: Some("test-agent".to_string()),
            task: "Complete task".to_string(),
            depends_on: vec!["dep1".to_string(), "dep2".to_string()],
            condition: Some(StepCondition::OnSuccess),
            timeout: 600,
            retry: RetryConfig {
                max_attempts: 3,
                delay: 10,
            },
            output: Some("result".to_string()),
        };

        assert_eq!(step.id, "complete-step");
        assert_eq!(step.agent, Some("test-agent".to_string()));
        assert_eq!(step.depends_on.len(), 2);
        assert_eq!(step.timeout, 600);
        assert_eq!(step.retry.max_attempts, 3);
    }

    #[test]
    fn test_step_condition_variants() {
        let always = StepCondition::Always;
        let on_success = StepCondition::OnSuccess;
        let on_failure = StepCondition::OnFailure;
        let if_expr = StepCondition::If {
            expression: "true".to_string(),
        };

        assert!(matches!(always, StepCondition::Always));
        assert!(matches!(on_success, StepCondition::OnSuccess));
        assert!(matches!(on_failure, StepCondition::OnFailure));
        assert!(matches!(if_expr, StepCondition::If { .. }));
    }

    #[test]
    fn test_failure_policy_variants() {
        let stop = FailurePolicy::Stop;
        let continue_policy = FailurePolicy::Continue;
        let cleanup = FailurePolicy::Cleanup;

        assert!(matches!(stop, FailurePolicy::Stop));
        assert!(matches!(continue_policy, FailurePolicy::Continue));
        assert!(matches!(cleanup, FailurePolicy::Cleanup));
    }
}
