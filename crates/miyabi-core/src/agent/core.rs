//! Agent struct and execution loop

use std::time::Duration;

use serde_json::Value;
use tokio::sync::mpsc;

use crate::{AnthropicClient, ContentBlock, Message, Role, StopReason};

use super::{AgentError, AgentEvent, AgentResult, ExecutorRegistry};

/// Configuration for agent execution
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Maximum number of iterations (default: 10)
    pub max_iterations: usize,
    /// Maximum tokens per turn (default: 8192)
    pub max_tokens_per_turn: u32,
    /// Timeout per tool execution (default: 300s)
    pub timeout_per_tool: Duration,
    /// Whether to require approval for tools (default: true)
    pub require_approval: bool,
    /// Patterns for auto-approved tools (e.g., "read", "glob")
    pub auto_approve_patterns: Vec<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            max_tokens_per_turn: 8192,
            timeout_per_tool: Duration::from_secs(300),
            require_approval: true,
            auto_approve_patterns: vec!["read".to_string(), "glob".to_string(), "grep".to_string()],
        }
    }
}

/// Autonomous agent for executing tasks with tools
pub struct Agent {
    client: AnthropicClient,
    executor_registry: ExecutorRegistry,
    config: AgentConfig,
    system_prompt: Option<String>,
    event_tx: Option<mpsc::Sender<AgentEvent>>,
}

impl Agent {
    /// Create new agent with client and tools
    pub fn new(client: AnthropicClient, executor_registry: ExecutorRegistry) -> Self {
        Self {
            client,
            executor_registry,
            config: AgentConfig::default(),
            system_prompt: None,
            event_tx: None,
        }
    }

    /// Set agent configuration
    pub fn with_config(mut self, config: AgentConfig) -> Self {
        self.config = config;
        self
    }

    /// Set system prompt
    pub fn with_system_prompt(mut self, prompt: String) -> Self {
        self.system_prompt = Some(prompt);
        self
    }

    /// Set event channel for progress updates
    pub fn with_event_channel(mut self, tx: mpsc::Sender<AgentEvent>) -> Self {
        self.event_tx = Some(tx);
        self
    }

    /// Emit event if channel is set
    async fn emit_event(&self, event: AgentEvent) {
        if let Some(tx) = &self.event_tx {
            let _ = tx.send(event).await;
        }
    }

    /// Check if tool should be auto-approved
    fn is_auto_approved(&self, tool_name: &str) -> bool {
        if !self.config.require_approval {
            return true;
        }
        self.config
            .auto_approve_patterns
            .iter()
            .any(|pattern| tool_name.to_lowercase().contains(&pattern.to_lowercase()))
    }

    /// Extract text content from response
    fn extract_text(&self, content: &[ContentBlock]) -> String {
        content
            .iter()
            .filter_map(|block| {
                if let ContentBlock::Text { text } = block {
                    Some(text.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Extract tool uses from response
    fn extract_tool_uses(&self, content: &[ContentBlock]) -> Vec<ToolUse> {
        content
            .iter()
            .filter_map(|block| {
                if let ContentBlock::ToolUse { id, name, input } = block {
                    Some(ToolUse {
                        id: id.clone(),
                        name: name.clone(),
                        input: input.clone(),
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Main agent execution loop
    pub async fn run(&self, prompt: &str) -> Result<AgentResult, AgentError> {
        self.emit_event(AgentEvent::Started {
            prompt: prompt.to_string(),
        })
        .await;

        let mut messages = vec![Message {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: prompt.to_string(),
            }],
        }];

        let mut total_tokens = 0usize;
        let mut tool_calls = 0usize;

        let tools = self.executor_registry.to_api_tools();

        for iteration in 0..self.config.max_iterations {
            self.emit_event(AgentEvent::Thinking { iteration }).await;

            // Call Claude API
            let response = self
                .client
                .message(
                    messages.clone(),
                    self.system_prompt.clone(),
                    Some(tools.clone()),
                    None,
                )
                .await?;

            // Track token usage
            let input_tokens = response.usage.input_tokens as usize;
            let output_tokens = response.usage.output_tokens as usize;
            total_tokens += input_tokens + output_tokens;

            self.emit_event(AgentEvent::TokenUsage {
                input: input_tokens,
                output: output_tokens,
            })
            .await;

            // Handle response based on stop reason
            match response.stop_reason {
                Some(StopReason::EndTurn) | Some(StopReason::StopSequence) => {
                    // Agent completed
                    let output = self.extract_text(&response.content);
                    let result = AgentResult {
                        output,
                        iterations: iteration + 1,
                        tool_calls,
                        total_tokens,
                        messages,
                    };
                    self.emit_event(AgentEvent::Completed {
                        result: result.clone(),
                    })
                    .await;
                    return Ok(result);
                }
                Some(StopReason::ToolUse) => {
                    // Extract tool uses
                    let tool_uses = self.extract_tool_uses(&response.content);
                    let mut results = Vec::new();

                    for tool_use in tool_uses {
                        tool_calls += 1;

                        self.emit_event(AgentEvent::ToolDetected {
                            id: tool_use.id.clone(),
                            name: tool_use.name.clone(),
                            input: tool_use.input.clone(),
                        })
                        .await;

                        // Check approval
                        let needs_approval = !self.is_auto_approved(&tool_use.name)
                            && self.executor_registry.requires_approval(&tool_use.name);

                        if needs_approval {
                            self.emit_event(AgentEvent::AwaitingApproval {
                                id: tool_use.id.clone(),
                                name: tool_use.name.clone(),
                                input: tool_use.input.clone(),
                            })
                            .await;
                            // In non-interactive mode, auto-approve for now
                            // TODO: Add approval callback mechanism
                        }

                        // Execute tool
                        self.emit_event(AgentEvent::ToolExecuting {
                            name: tool_use.name.clone(),
                            input: tool_use.input.clone(),
                        })
                        .await;

                        match self
                            .executor_registry
                            .execute(&tool_use.name, tool_use.input.clone())
                            .await
                        {
                            Ok(output) => {
                                self.emit_event(AgentEvent::ToolCompleted {
                                    name: tool_use.name.clone(),
                                    output: output.clone(),
                                })
                                .await;

                                // Create tool result
                                let content = serde_json::to_string(&output.content)
                                    .unwrap_or_else(|_| output.content.to_string());

                                results.push(ContentBlock::ToolResult {
                                    tool_use_id: tool_use.id,
                                    content,
                                });
                            }
                            Err(e) => {
                                self.emit_event(AgentEvent::ToolFailed {
                                    name: tool_use.name.clone(),
                                    error: e.to_string(),
                                })
                                .await;

                                // Add error as tool result
                                results.push(ContentBlock::ToolResult {
                                    tool_use_id: tool_use.id,
                                    content: format!("Error: {}", e),
                                });
                            }
                        }
                    }

                    // Add assistant message with tool uses
                    messages.push(Message {
                        role: Role::Assistant,
                        content: response.content,
                    });

                    // Add tool results as user message
                    messages.push(Message {
                        role: Role::User,
                        content: results,
                    });
                }
                Some(StopReason::MaxTokens) => {
                    return Err(AgentError::MaxTokensReached);
                }
                Some(StopReason::ModelContextWindowExceeded) => {
                    return Err(AgentError::ContextWindowExceeded);
                }
                None => {
                    // No stop reason, treat as end turn
                    let output = self.extract_text(&response.content);
                    let result = AgentResult {
                        output,
                        iterations: iteration + 1,
                        tool_calls,
                        total_tokens,
                        messages,
                    };
                    self.emit_event(AgentEvent::Completed {
                        result: result.clone(),
                    })
                    .await;
                    return Ok(result);
                }
            }
        }

        Err(AgentError::MaxIterationsReached(self.config.max_iterations))
    }
}

/// Internal tool use representation
#[derive(Debug, Clone)]
struct ToolUse {
    id: String,
    name: String,
    input: Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_config_default() {
        let config = AgentConfig::default();
        assert_eq!(config.max_iterations, 10);
        assert_eq!(config.max_tokens_per_turn, 8192);
        assert!(config.require_approval);
        assert!(config.auto_approve_patterns.contains(&"read".to_string()));
    }

    #[test]
    fn test_is_auto_approved() {
        let client = AnthropicClient::new("test-key".to_string()).unwrap();
        let registry = ExecutorRegistry::new();
        let agent = Agent::new(client, registry);

        assert!(agent.is_auto_approved("read"));
        assert!(agent.is_auto_approved("glob"));
        assert!(agent.is_auto_approved("grep"));
        assert!(!agent.is_auto_approved("bash"));
        assert!(!agent.is_auto_approved("write"));
    }

    #[test]
    fn test_extract_text() {
        let client = AnthropicClient::new("test-key".to_string()).unwrap();
        let registry = ExecutorRegistry::new();
        let agent = Agent::new(client, registry);

        let content = vec![
            ContentBlock::Text {
                text: "Hello".to_string(),
            },
            ContentBlock::Text {
                text: "World".to_string(),
            },
        ];

        let text = agent.extract_text(&content);
        assert_eq!(text, "Hello\nWorld");
    }
}
