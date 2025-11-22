//! Miyabi Core - Shared types and utilities
//!
//! This crate provides core types and utilities shared across the Miyabi framework.

pub mod agent;
pub mod anthropic;
pub mod config;
pub mod conversation;
pub mod error;
pub mod session;
pub mod token;
pub mod tool;
pub mod tools;
pub mod types;

pub use agent::{
    Agent, AgentConfig, AgentError, AgentEvent, AgentResult, ExecutorRegistry, RiskLevel,
    ToolExecutor,
};
pub use anthropic::{
    AnthropicClient,
    AnthropicError,
    ContentBlock,
    Message,
    MessagesRequest,
    MessagesResponse,
    RetryConfig, // Retry configuration for API requests
    Role,
    StopReason,
    StreamEvent,
    Tool as ApiTool, // Anthropic API tool definition format
    Usage,
};
pub use config::{ApiConfig, Config, SessionConfig, ToolConfig, UiConfig};
pub use conversation::{
    Conversation, ConversationError, ConversationManager, ConversationMessage, ConversationMetadata,
};
pub use error::Error;
pub use session::{Session, SessionMetadata, SessionStorage};
pub use token::{ContextManager, ContextUsage, ModelLimits, TokenCounter, TokenUsage};
pub use tool::{ParameterDef, Tool as ToolTrait, ToolError, ToolOutput, ToolRegistry, ToolResult};
pub use tools::{
    create_file_tool_registry, create_standard_tool_registry, BashTool, EditTool, GlobTool,
    GrepTool, ReadTool, WriteTool,
};
pub use types::*;
