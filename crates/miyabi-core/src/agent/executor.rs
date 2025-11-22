//! Tool executor trait and registry

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;

use crate::{
    anthropic::Tool as ApiTool,
    tool::{Tool as ToolTrait, ToolError, ToolOutput},
    tools::{BashTool, EditTool, GlobTool, GrepTool, ReadTool, WriteTool},
};

/// Risk level for tool execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    /// Read-only operations (Read, Glob, Grep)
    Low,
    /// File modification (Write, Edit)
    Medium,
    /// System execution (Bash)
    High,
    /// Destructive operations
    Critical,
}

impl RiskLevel {
    /// Check if this risk level requires approval
    pub fn requires_approval(&self) -> bool {
        matches!(self, RiskLevel::High | RiskLevel::Critical)
    }

    /// Get display name
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskLevel::Low => "Low",
            RiskLevel::Medium => "Medium",
            RiskLevel::High => "High",
            RiskLevel::Critical => "Critical",
        }
    }
}

/// Trait for executable tools with metadata
#[async_trait]
pub trait ToolExecutor: Send + Sync {
    /// Tool name
    fn name(&self) -> &str;

    /// Tool description
    fn description(&self) -> &str;

    /// Tool definition for API
    fn definition(&self) -> ApiTool;

    /// Risk level of this tool
    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Medium
    }

    /// Execute the tool with input
    async fn execute(&self, input: Value) -> Result<ToolOutput, ToolError>;

    /// Validate input before execution
    fn validate(&self, _input: &Value) -> Result<(), ToolError> {
        Ok(())
    }

    /// Timeout for this tool
    fn timeout(&self) -> Duration {
        Duration::from_secs(120)
    }
}

/// Adapter to wrap existing Tool trait implementations
pub struct ToolExecutorAdapter<T: ToolTrait> {
    tool: T,
    risk_level: RiskLevel,
}

impl<T: ToolTrait> ToolExecutorAdapter<T> {
    pub fn new(tool: T, risk_level: RiskLevel) -> Self {
        Self { tool, risk_level }
    }
}

#[async_trait]
impl<T: ToolTrait + Send + Sync + 'static> ToolExecutor for ToolExecutorAdapter<T> {
    fn name(&self) -> &str {
        self.tool.name()
    }

    fn description(&self) -> &str {
        self.tool.description()
    }

    fn definition(&self) -> ApiTool {
        ApiTool {
            name: self.tool.name().to_string(),
            description: self.tool.description().to_string(),
            input_schema: self.tool.schema(),
        }
    }

    fn risk_level(&self) -> RiskLevel {
        self.risk_level
    }

    async fn execute(&self, input: Value) -> Result<ToolOutput, ToolError> {
        self.tool.execute(input).await
    }

    fn validate(&self, input: &Value) -> Result<(), ToolError> {
        self.tool.validate(input)
    }
}

/// Registry for tool executors
pub struct ExecutorRegistry {
    executors: HashMap<String, Arc<dyn ToolExecutor>>,
}

impl ExecutorRegistry {
    /// Create empty registry
    pub fn new() -> Self {
        Self {
            executors: HashMap::new(),
        }
    }

    /// Register a tool executor
    pub fn register<T: ToolExecutor + 'static>(&mut self, executor: T) {
        let name = executor.name().to_string();
        self.executors.insert(name, Arc::new(executor));
    }

    /// Get executor by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn ToolExecutor>> {
        self.executors.get(name).cloned()
    }

    /// Execute tool by name
    pub async fn execute(&self, name: &str, input: Value) -> Result<ToolOutput, ToolError> {
        let executor = self
            .executors
            .get(name)
            .ok_or_else(|| ToolError::NotFound(name.to_string()))?;

        executor.validate(&input)?;
        executor.execute(input).await
    }

    /// Get all tool definitions for API
    pub fn to_api_tools(&self) -> Vec<ApiTool> {
        self.executors.values().map(|e| e.definition()).collect()
    }

    /// Get tool names
    pub fn tool_names(&self) -> Vec<String> {
        self.executors.keys().cloned().collect()
    }

    /// Get risk level for a tool
    pub fn risk_level(&self, name: &str) -> Option<RiskLevel> {
        self.executors.get(name).map(|e| e.risk_level())
    }

    /// Check if tool requires approval
    pub fn requires_approval(&self, name: &str) -> bool {
        self.executors
            .get(name)
            .map(|e| e.risk_level().requires_approval())
            .unwrap_or(true)
    }

    /// Create registry with standard tools
    pub fn with_standard_tools() -> Self {
        let mut registry = Self::new();

        // Read-only tools (Low risk)
        registry.register(ToolExecutorAdapter::new(ReadTool::new(), RiskLevel::Low));
        registry.register(ToolExecutorAdapter::new(GlobTool::new(), RiskLevel::Low));
        registry.register(ToolExecutorAdapter::new(GrepTool::new(), RiskLevel::Low));

        // File modification tools (Medium risk)
        registry.register(ToolExecutorAdapter::new(
            WriteTool::new(),
            RiskLevel::Medium,
        ));
        registry.register(ToolExecutorAdapter::new(EditTool::new(), RiskLevel::Medium));

        // System execution (High risk)
        registry.register(ToolExecutorAdapter::new(BashTool::new(), RiskLevel::High));

        registry
    }
}

impl Default for ExecutorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ExecutorRegistry {
    fn clone(&self) -> Self {
        Self {
            executors: self.executors.clone(),
        }
    }
}
