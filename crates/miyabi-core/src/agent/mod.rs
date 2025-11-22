//! Agent module for autonomous task execution
//!
//! This module provides the core Agent loop that enables autonomous
//! tool execution with Claude API, similar to Claude Agent SDK.

mod approval;
mod core;
mod events;
mod executor;

pub use approval::{
    ApprovalCallback, ApprovalDecision, ApprovalRequest, AutoApproveAll, ChannelApprover,
    RejectHighRisk,
};
pub use core::{Agent, AgentConfig};
pub use events::{AgentError, AgentEvent, AgentResult};
pub use executor::{ExecutorRegistry, RiskLevel, ToolExecutor};
