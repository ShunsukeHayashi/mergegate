//! Tool executor trait and registry

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;

use crate::{
    anthropic::Tool as ApiTool,
    github::GitHubClient,
    github_tools::{
        AddCommentTool, AddLabelsTool, CreateIssueTool, CreatePullRequestTool, GetIssueTool,
        ListIssuesTool, ListPullRequestsTool,
    },
    mcp::{McpManager, McpTool},
    tool::{Tool as ToolTrait, ToolError, ToolOutput},
    tools::{BashTool, EditTool, GlobTool, GrepTool, ReadTool, WriteTool},
};
use tokio::sync::Mutex as AsyncMutex;

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

/// Executor for MCP tools
pub struct McpToolExecutor {
    server_name: String,
    tool_name: String,
    tool_description: String,
    input_schema: Value,
    manager: Arc<AsyncMutex<McpManager>>,
}

impl McpToolExecutor {
    pub fn new(server_name: String, tool: McpTool, manager: Arc<AsyncMutex<McpManager>>) -> Self {
        Self {
            server_name,
            tool_name: tool.name,
            tool_description: tool.description,
            input_schema: tool.input_schema.unwrap_or(serde_json::json!({
                "type": "object",
                "properties": {}
            })),
            manager,
        }
    }
}

#[async_trait]
impl ToolExecutor for McpToolExecutor {
    fn name(&self) -> &str {
        &self.tool_name
    }

    fn description(&self) -> &str {
        &self.tool_description
    }

    fn definition(&self) -> ApiTool {
        ApiTool {
            name: self.tool_name.clone(),
            description: self.tool_description.clone(),
            input_schema: self.input_schema.clone(),
        }
    }

    fn risk_level(&self) -> RiskLevel {
        // MCP tools are external, treat as medium risk by default
        RiskLevel::Medium
    }

    async fn execute(&self, input: Value) -> Result<ToolOutput, ToolError> {
        let mut manager = self.manager.lock().await;

        match manager
            .call_tool(&self.server_name, &self.tool_name, input)
            .await
        {
            Ok(result) => {
                let content =
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string());
                Ok(ToolOutput::success(content))
            }
            Err(e) => Err(ToolError::ExecutionFailed(e.to_string())),
        }
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

    /// Register MCP tools from an MCP manager
    pub async fn register_mcp_tools(
        &mut self,
        manager: Arc<AsyncMutex<McpManager>>,
    ) -> Result<usize, ToolError> {
        let mut count = 0;

        // Get all tools from all servers
        let all_tools = {
            let mut mgr = manager.lock().await;
            mgr.list_all_tools()
                .await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?
        };

        for (server_name, tools) in all_tools {
            for tool in tools {
                let executor = McpToolExecutor::new(server_name.clone(), tool, manager.clone());
                self.register(executor);
                count += 1;
            }
        }

        Ok(count)
    }

    /// Register GitHub tools with the registry
    pub fn register_github_tools(&mut self, client: GitHubClient) {
        // Read-only tools (Low risk)
        self.register(ToolExecutorAdapter::new(
            ListIssuesTool::new(client.clone()),
            RiskLevel::Low,
        ));
        self.register(ToolExecutorAdapter::new(
            GetIssueTool::new(client.clone()),
            RiskLevel::Low,
        ));
        self.register(ToolExecutorAdapter::new(
            ListPullRequestsTool::new(client.clone()),
            RiskLevel::Low,
        ));

        // Modification tools (Medium risk)
        self.register(ToolExecutorAdapter::new(
            CreateIssueTool::new(client.clone()),
            RiskLevel::Medium,
        ));
        self.register(ToolExecutorAdapter::new(
            AddCommentTool::new(client.clone()),
            RiskLevel::Medium,
        ));
        self.register(ToolExecutorAdapter::new(
            AddLabelsTool::new(client.clone()),
            RiskLevel::Medium,
        ));
        self.register(ToolExecutorAdapter::new(
            CreatePullRequestTool::new(client),
            RiskLevel::Medium,
        ));
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
