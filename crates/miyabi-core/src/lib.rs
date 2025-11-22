//! Miyabi Core - Shared types and utilities
//!
//! This crate provides core types and utilities shared across the Miyabi framework.

pub mod agent;
pub mod anthropic;
pub mod cache;
pub mod config;
pub mod conversation;
pub mod error;
pub mod error_policy;
pub mod feature_flags;
pub mod git;
pub mod github;
pub mod github_tools;
pub mod logger;
pub mod plugin;
pub mod retry;
pub mod rules;
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
pub use cache::{
    create_api_cache, create_llm_cache, ApiCache, ApiCacheKey, CacheEntry, CacheStats, LLMCache,
    LLMCacheKey, TTLCache,
};
pub use error::Error;
pub use error_policy::{CircuitBreaker, CircuitState, FallbackStrategy};
pub use feature_flags::{FeatureFlag, FeatureFlagManager};
pub use git::{
    find_git_root, get_current_branch, get_head_short_hash, get_main_branch,
    has_uncommitted_changes, is_in_git_repo, is_valid_repository,
};
pub use github::{
    Comment, CreateIssueRequest, CreatePullRequestRequest, GitHubClient, Issue, Label,
    PullRequest, User,
};
pub use github_tools::{
    create_github_tool_registry, AddCommentTool, AddLabelsTool, CreateIssueTool,
    CreatePullRequestTool, GetIssueTool, ListIssuesTool, ListPullRequestsTool,
};
pub use logger::{init_logger, init_logger_with_config, LogFormat, LogLevel, LoggerConfig};
pub use plugin::{Plugin, PluginContext, PluginManager, PluginMetadata, PluginResult, PluginState};
pub use retry::{retry_with_backoff, RetryConfig as BackoffRetryConfig};
pub use rules::{AgentPreferences, MiyabiRules, Rule, RulesError, RulesLoader};
pub use session::{Session, SessionMetadata, SessionStorage};
pub use token::{ContextManager, ContextUsage, ModelLimits, TokenCounter, TokenUsage};
pub use tool::{ParameterDef, Tool as ToolTrait, ToolError, ToolOutput, ToolRegistry, ToolResult};
pub use tools::{
    create_file_tool_registry, create_standard_tool_registry, BashTool, EditTool, GlobTool,
    GrepTool, ReadTool, WriteTool,
};
pub use types::*;
