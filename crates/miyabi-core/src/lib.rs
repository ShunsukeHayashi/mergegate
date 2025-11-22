//! Miyabi Core - Shared types and utilities
//!
//! This crate provides core types and utilities shared across the Miyabi framework.

pub mod error;
pub mod types;
pub mod anthropic;
pub mod tool;
pub mod conversation;
pub mod tools;
pub mod token;

pub use error::Error;
pub use types::*;
pub use anthropic::{
    AnthropicClient, AnthropicError, Message, Role, ContentBlock,
    MessagesRequest, MessagesResponse, StreamEvent, StopReason, Usage,
};
// Note: anthropic::Tool is a different type from tool::Tool trait
pub use tool::{
    Tool as ToolTrait, ToolRegistry, ToolError, ToolOutput, ToolResult, ParameterDef,
};
pub use conversation::{
    Conversation, ConversationMessage, ConversationManager, ConversationMetadata, ConversationError,
};
pub use tools::{ReadTool, WriteTool, EditTool, create_file_tool_registry};
pub use token::{TokenCounter, TokenUsage, ContextManager, ContextUsage, ModelLimits};
