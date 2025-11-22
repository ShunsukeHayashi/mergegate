//! Multi-Agent Orchestration
//!
//! Provides parallel execution capabilities for multiple Agent instances.

use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;

use crate::agent::{Agent, AgentConfig, ExecutorRegistry};
use crate::anthropic::AnthropicClient;

/// Configuration for parallel execution
#[derive(Debug, Clone)]
pub struct ParallelConfig {
    /// Maximum number of concurrent agents
    pub max_concurrency: usize,
    /// Timeout for individual agent execution in seconds
    pub timeout_seconds: u64,
    /// Whether to fail fast on first error
    pub fail_fast: bool,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            max_concurrency: 4,
            timeout_seconds: 300, // 5 minutes
            fail_fast: false,
        }
    }
}

/// Result of parallel execution
#[derive(Debug, Clone)]
pub struct ParallelResult {
    /// Individual agent results
    pub results: Vec<TaskResult>,
    /// Total execution time in milliseconds
    pub total_duration_ms: u64,
    /// Number of successful executions
    pub success_count: usize,
    /// Number of failed executions
    pub failure_count: usize,
}

impl ParallelResult {
    /// Check if all executions were successful
    pub fn all_successful(&self) -> bool {
        self.failure_count == 0
    }

    /// Get success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.results.is_empty() {
            0.0
        } else {
            (self.success_count as f64 / self.results.len() as f64) * 100.0
        }
    }

    /// Get total tokens used across all agents
    pub fn total_tokens(&self) -> usize {
        self.results.iter().map(|r| r.tokens_used).sum()
    }
}

/// Result from a single task execution
#[derive(Debug, Clone)]
pub struct TaskResult {
    /// Task ID
    pub task_id: String,
    /// Whether the task succeeded
    pub success: bool,
    /// Output from the agent
    pub output: String,
    /// Error message if failed
    pub error: Option<String>,
    /// Tokens used
    pub tokens_used: usize,
    /// Execution time in milliseconds
    pub duration_ms: u64,
}

/// A task to be executed by an agent
#[derive(Debug, Clone)]
pub struct OrchestratorTask {
    /// Unique task ID
    pub id: String,
    /// Prompt for the agent
    pub prompt: String,
    /// Optional system prompt override
    pub system_prompt: Option<String>,
}

impl OrchestratorTask {
    /// Create a new task
    pub fn new(id: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            prompt: prompt.into(),
            system_prompt: None,
        }
    }

    /// Set system prompt
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }
}

/// Parallel executor for multiple agents
pub struct Orchestrator {
    config: ParallelConfig,
    semaphore: Arc<Semaphore>,
    client: AnthropicClient,
    registry: ExecutorRegistry,
    agent_config: AgentConfig,
}

impl Orchestrator {
    /// Create a new orchestrator
    pub fn new(
        client: AnthropicClient,
        registry: ExecutorRegistry,
        config: ParallelConfig,
    ) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_concurrency));
        Self {
            config,
            semaphore,
            client,
            registry,
            agent_config: AgentConfig::default(),
        }
    }

    /// Set agent configuration
    pub fn with_agent_config(mut self, config: AgentConfig) -> Self {
        self.agent_config = config;
        self
    }

    /// Execute multiple tasks in parallel
    pub async fn execute(&self, tasks: Vec<OrchestratorTask>) -> ParallelResult {
        let start_time = std::time::Instant::now();
        let mut handles: Vec<JoinHandle<TaskResult>> = Vec::new();

        for task in tasks {
            let semaphore = self.semaphore.clone();
            let client = self.client.clone();
            let registry = self.registry.clone();
            let agent_config = self.agent_config.clone();
            let timeout_seconds = self.config.timeout_seconds;

            let handle = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.expect("Semaphore closed");
                let task_start = std::time::Instant::now();

                // Create agent for this task
                let mut agent = Agent::new(client, registry).with_config(agent_config);

                if let Some(system_prompt) = &task.system_prompt {
                    agent = agent.with_system_prompt(system_prompt.clone());
                }

                // Execute with timeout
                let result = tokio::time::timeout(
                    std::time::Duration::from_secs(timeout_seconds),
                    agent.run(&task.prompt),
                )
                .await;

                let duration_ms = task_start.elapsed().as_millis() as u64;

                match result {
                    Ok(Ok(agent_result)) => TaskResult {
                        task_id: task.id,
                        success: true,
                        output: agent_result.output,
                        error: None,
                        tokens_used: agent_result.total_tokens,
                        duration_ms,
                    },
                    Ok(Err(e)) => TaskResult {
                        task_id: task.id,
                        success: false,
                        output: String::new(),
                        error: Some(e.to_string()),
                        tokens_used: 0,
                        duration_ms,
                    },
                    Err(_) => TaskResult {
                        task_id: task.id,
                        success: false,
                        output: String::new(),
                        error: Some(format!("Timeout after {} seconds", timeout_seconds)),
                        tokens_used: 0,
                        duration_ms,
                    },
                }
            });

            handles.push(handle);
        }

        // Wait for all tasks to complete
        let results: Vec<TaskResult> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|result| result.expect("Task panicked"))
            .collect();

        let total_duration_ms = start_time.elapsed().as_millis() as u64;
        let success_count = results.iter().filter(|r| r.success).count();
        let failure_count = results.len() - success_count;

        ParallelResult {
            results,
            total_duration_ms,
            success_count,
            failure_count,
        }
    }

    /// Execute tasks sequentially (for comparison/debugging)
    pub async fn execute_sequential(&self, tasks: Vec<OrchestratorTask>) -> ParallelResult {
        let start_time = std::time::Instant::now();
        let mut results = Vec::new();

        for task in tasks {
            let task_start = std::time::Instant::now();

            // Create agent for this task
            let mut agent = Agent::new(self.client.clone(), self.registry.clone())
                .with_config(self.agent_config.clone());

            if let Some(system_prompt) = &task.system_prompt {
                agent = agent.with_system_prompt(system_prompt.clone());
            }

            // Execute with timeout
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(self.config.timeout_seconds),
                agent.run(&task.prompt),
            )
            .await;

            let duration_ms = task_start.elapsed().as_millis() as u64;

            let task_result = match result {
                Ok(Ok(agent_result)) => TaskResult {
                    task_id: task.id,
                    success: true,
                    output: agent_result.output,
                    error: None,
                    tokens_used: agent_result.total_tokens,
                    duration_ms,
                },
                Ok(Err(e)) => TaskResult {
                    task_id: task.id,
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                    tokens_used: 0,
                    duration_ms,
                },
                Err(_) => TaskResult {
                    task_id: task.id,
                    success: false,
                    output: String::new(),
                    error: Some(format!(
                        "Timeout after {} seconds",
                        self.config.timeout_seconds
                    )),
                    tokens_used: 0,
                    duration_ms,
                },
            };

            // Fail fast if configured
            if self.config.fail_fast && !task_result.success {
                results.push(task_result);
                break;
            }

            results.push(task_result);
        }

        let total_duration_ms = start_time.elapsed().as_millis() as u64;
        let success_count = results.iter().filter(|r| r.success).count();
        let failure_count = results.len() - success_count;

        ParallelResult {
            results,
            total_duration_ms,
            success_count,
            failure_count,
        }
    }
}

/// Create a default orchestrator
pub fn create_orchestrator(client: AnthropicClient, registry: ExecutorRegistry) -> Orchestrator {
    Orchestrator::new(client, registry, ParallelConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_config_default() {
        let config = ParallelConfig::default();
        assert_eq!(config.max_concurrency, 4);
        assert_eq!(config.timeout_seconds, 300);
        assert!(!config.fail_fast);
    }

    #[test]
    fn test_orchestrator_task_creation() {
        let task = OrchestratorTask::new("task-1", "Do something");
        assert_eq!(task.id, "task-1");
        assert_eq!(task.prompt, "Do something");
        assert!(task.system_prompt.is_none());
    }

    #[test]
    fn test_orchestrator_task_with_system_prompt() {
        let task = OrchestratorTask::new("task-1", "Do something")
            .with_system_prompt("You are helpful");
        assert_eq!(task.system_prompt, Some("You are helpful".to_string()));
    }

    #[test]
    fn test_parallel_result_all_successful() {
        let result = ParallelResult {
            results: vec![
                TaskResult {
                    task_id: "1".to_string(),
                    success: true,
                    output: "Done".to_string(),
                    error: None,
                    tokens_used: 100,
                    duration_ms: 1000,
                },
                TaskResult {
                    task_id: "2".to_string(),
                    success: true,
                    output: "Done".to_string(),
                    error: None,
                    tokens_used: 100,
                    duration_ms: 1000,
                },
            ],
            total_duration_ms: 1500,
            success_count: 2,
            failure_count: 0,
        };

        assert!(result.all_successful());
        assert_eq!(result.success_rate(), 100.0);
        assert_eq!(result.total_tokens(), 200);
    }

    #[test]
    fn test_parallel_result_partial_success() {
        let result = ParallelResult {
            results: vec![
                TaskResult {
                    task_id: "1".to_string(),
                    success: true,
                    output: "Done".to_string(),
                    error: None,
                    tokens_used: 100,
                    duration_ms: 1000,
                },
                TaskResult {
                    task_id: "2".to_string(),
                    success: false,
                    output: String::new(),
                    error: Some("Failed".to_string()),
                    tokens_used: 50,
                    duration_ms: 500,
                },
            ],
            total_duration_ms: 1500,
            success_count: 1,
            failure_count: 1,
        };

        assert!(!result.all_successful());
        assert_eq!(result.success_rate(), 50.0);
        assert_eq!(result.total_tokens(), 150);
    }

    #[test]
    fn test_parallel_result_empty() {
        let result = ParallelResult {
            results: vec![],
            total_duration_ms: 0,
            success_count: 0,
            failure_count: 0,
        };

        assert!(result.all_successful());
        assert_eq!(result.success_rate(), 0.0);
        assert_eq!(result.total_tokens(), 0);
    }

    #[test]
    fn test_task_result_creation() {
        let result = TaskResult {
            task_id: "test".to_string(),
            success: true,
            output: "Output".to_string(),
            error: None,
            tokens_used: 500,
            duration_ms: 2000,
        };

        assert_eq!(result.task_id, "test");
        assert!(result.success);
        assert_eq!(result.tokens_used, 500);
    }

    #[test]
    fn test_orchestrator_creation() {
        let client = AnthropicClient::new("test-key".to_string()).unwrap();
        let registry = ExecutorRegistry::with_standard_tools();
        let config = ParallelConfig {
            max_concurrency: 2,
            timeout_seconds: 60,
            fail_fast: true,
        };

        let orchestrator = Orchestrator::new(client, registry, config);
        assert_eq!(orchestrator.config.max_concurrency, 2);
        assert_eq!(orchestrator.config.timeout_seconds, 60);
        assert!(orchestrator.config.fail_fast);
    }

    #[test]
    fn test_create_orchestrator_helper() {
        let client = AnthropicClient::new("test-key".to_string()).unwrap();
        let registry = ExecutorRegistry::with_standard_tools();

        let orchestrator = create_orchestrator(client, registry);
        assert_eq!(orchestrator.config.max_concurrency, 4);
    }
}
