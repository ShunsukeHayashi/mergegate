//! Agent events and results

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::{Message, ToolOutput};

/// Events emitted during agent execution
#[derive(Debug, Clone)]
pub enum AgentEvent {
    // Lifecycle events
    /// Agent has started processing
    Started { prompt: String },
    /// Agent is thinking (new iteration)
    Thinking { iteration: usize },
    /// Agent completed successfully
    Completed { result: AgentResult },
    /// Agent failed with error
    Failed { error: String },

    // Tool events
    /// Tool use detected in response
    ToolDetected {
        id: String,
        name: String,
        input: Value,
    },
    /// Waiting for user approval
    AwaitingApproval {
        id: String,
        name: String,
        input: Value,
    },
    /// Tool was approved
    ToolApproved { id: String },
    /// Tool was denied
    ToolDenied { id: String, reason: String },
    /// Tool execution started
    ToolExecuting { name: String, input: Value },
    /// Tool execution progress
    ToolProgress { name: String, progress: f32 },
    /// Tool completed successfully
    ToolCompleted { name: String, output: ToolOutput },
    /// Tool execution failed
    ToolFailed { name: String, error: String },

    // Streaming events
    /// Text delta from streaming response
    TextDelta { text: String },
    /// Token usage update
    TokenUsage { input: usize, output: usize },
}

/// Result of agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    /// Final output text
    pub output: String,
    /// Number of iterations executed
    pub iterations: usize,
    /// Total tool calls made
    pub tool_calls: usize,
    /// Total tokens used
    pub total_tokens: usize,
    /// Final conversation state
    pub messages: Vec<Message>,
}

/// Agent execution errors
#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Maximum iterations ({0}) reached")]
    MaxIterationsReached(usize),

    #[error("Maximum tokens reached")]
    MaxTokensReached,

    #[error("Context window exceeded for this model")]
    ContextWindowExceeded,

    #[error("Tool execution failed: {0}")]
    ToolExecutionFailed(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Tool denied by user: {0}")]
    ToolDenied(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Timeout after {0} seconds")]
    Timeout(u64),

    #[error("Agent was cancelled")]
    Cancelled,
}

impl From<crate::AnthropicError> for AgentError {
    fn from(err: crate::AnthropicError) -> Self {
        AgentError::ApiError(err.to_string())
    }
}

impl From<serde_json::Error> for AgentError {
    fn from(err: serde_json::Error) -> Self {
        AgentError::SerializationError(err.to_string())
    }
}

impl From<crate::ToolError> for AgentError {
    fn from(err: crate::ToolError) -> Self {
        AgentError::ToolExecutionFailed(err.to_string())
    }
}
