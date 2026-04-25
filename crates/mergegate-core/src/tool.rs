//! Tool Trait and Registry System
//!
//! This module provides the core abstractions for defining and managing
//! tools that can be executed by AI agents.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Tool execution errors
#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Timeout: tool execution exceeded {0}ms")]
    Timeout(u64),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type for tool operations
pub type ToolResult<T> = std::result::Result<T, ToolError>;

/// Output from tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    /// Whether execution was successful
    pub success: bool,
    /// Output content
    pub content: Value,
    /// Error message if failed
    pub error: Option<String>,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
}

impl ToolOutput {
    /// Create a successful output
    pub fn success(content: impl Into<Value>) -> Self {
        Self {
            success: true,
            content: content.into(),
            error: None,
            duration_ms: 0,
        }
    }

    /// Create a failed output
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            success: false,
            content: Value::Null,
            error: Some(error.into()),
            duration_ms: 0,
        }
    }

    /// Set duration
    pub fn with_duration(mut self, ms: u64) -> Self {
        self.duration_ms = ms;
        self
    }

    /// Format output for display
    pub fn format_display(&self) -> String {
        if self.success {
            self.format_success()
        } else {
            self.format_error()
        }
    }

    /// Format successful output
    fn format_success(&self) -> String {
        match &self.content {
            Value::String(s) => s.clone(),
            Value::Object(obj) => {
                // Pretty print JSON object
                serde_json::to_string_pretty(&obj).unwrap_or_else(|_| format!("{:?}", obj))
            }
            Value::Array(arr) => {
                serde_json::to_string_pretty(&arr).unwrap_or_else(|_| format!("{:?}", arr))
            }
            Value::Null => "Success (no output)".to_string(),
            other => other.to_string(),
        }
    }

    /// Format error output
    fn format_error(&self) -> String {
        self.error
            .clone()
            .unwrap_or_else(|| "Unknown error".to_string())
    }

    /// Get a truncated summary of the output
    pub fn summary(&self, max_len: usize) -> String {
        let full = self.format_display();
        if full.len() <= max_len {
            full
        } else {
            format!("{}...", &full[..max_len.saturating_sub(3)])
        }
    }

    /// Get the content as a string if possible
    pub fn as_text(&self) -> Option<String> {
        match &self.content {
            Value::String(s) => Some(s.clone()),
            Value::Object(obj) => {
                // Check for common text fields
                obj.get("content")
                    .or_else(|| obj.get("text"))
                    .or_else(|| obj.get("output"))
                    .or_else(|| obj.get("stdout"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            }
            _ => None,
        }
    }

    /// Format duration for display
    pub fn format_duration(&self) -> String {
        if self.duration_ms < 1000 {
            format!("{}ms", self.duration_ms)
        } else {
            format!("{:.2}s", self.duration_ms as f64 / 1000.0)
        }
    }

    /// Get status indicator
    pub fn status_indicator(&self) -> &'static str {
        if self.success {
            "✓"
        } else {
            "✗"
        }
    }

    /// Check if the output contains an error code
    pub fn has_error_code(&self) -> bool {
        if let Value::Object(obj) = &self.content {
            obj.get("exit_code")
                .and_then(|v| v.as_i64())
                .map(|code| code != 0)
                .unwrap_or(false)
        } else {
            false
        }
    }
}

/// Parameter definition for tool input schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDef {
    /// Parameter name
    pub name: String,
    /// Parameter type (string, number, boolean, array, object)
    #[serde(rename = "type")]
    pub param_type: String,
    /// Parameter description
    pub description: String,
    /// Whether parameter is required
    #[serde(default)]
    pub required: bool,
    /// Default value if not provided
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    /// Enum values if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
}

impl ParameterDef {
    /// Create a required string parameter
    pub fn required_string(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            param_type: "string".to_string(),
            description: description.into(),
            required: true,
            default: None,
            enum_values: None,
        }
    }

    /// Create an optional string parameter
    pub fn optional_string(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            param_type: "string".to_string(),
            description: description.into(),
            required: false,
            default: None,
            enum_values: None,
        }
    }

    /// Create a required boolean parameter
    pub fn required_bool(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            param_type: "boolean".to_string(),
            description: description.into(),
            required: true,
            default: None,
            enum_values: None,
        }
    }

    /// Set default value
    pub fn with_default(mut self, default: impl Into<Value>) -> Self {
        self.default = Some(default.into());
        self.required = false;
        self
    }

    /// Set enum values
    pub fn with_enum(mut self, values: Vec<String>) -> Self {
        self.enum_values = Some(values);
        self
    }
}

/// Tool definition trait
///
/// Implement this trait to create a tool that can be executed by agents.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get the tool's unique name
    fn name(&self) -> &str;

    /// Get a human-readable description
    fn description(&self) -> &str;

    /// Get parameter definitions for the tool
    fn parameters(&self) -> Vec<ParameterDef>;

    /// Generate JSON schema for the tool input
    fn schema(&self) -> Value {
        let params = self.parameters();
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();

        for param in params {
            let mut prop = serde_json::Map::new();
            prop.insert("type".to_string(), Value::String(param.param_type.clone()));
            prop.insert(
                "description".to_string(),
                Value::String(param.description.clone()),
            );

            if let Some(default) = param.default {
                prop.insert("default".to_string(), default);
            }

            if let Some(enum_values) = param.enum_values {
                prop.insert(
                    "enum".to_string(),
                    Value::Array(enum_values.into_iter().map(Value::String).collect()),
                );
            }

            properties.insert(param.name.clone(), Value::Object(prop));

            if param.required {
                required.push(Value::String(param.name));
            }
        }

        serde_json::json!({
            "type": "object",
            "properties": properties,
            "required": required
        })
    }

    /// Execute the tool with given input
    async fn execute(&self, input: Value) -> ToolResult<ToolOutput>;

    /// Validate input before execution
    fn validate(&self, input: &Value) -> ToolResult<()> {
        let params = self.parameters();

        for param in params {
            if param.required && input.get(&param.name).is_none() {
                return Err(ToolError::ValidationError(format!(
                    "Required parameter '{}' is missing",
                    param.name
                )));
            }
        }

        Ok(())
    }
}

/// Tool registry for managing available tools
#[derive(Clone, Default)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool
    pub fn register<T: Tool + 'static>(&mut self, tool: T) -> &mut Self {
        let name = tool.name().to_string();
        info!("Registering tool: {}", name);
        self.tools.insert(name, Arc::new(tool));
        self
    }

    /// Register an Arc-wrapped tool
    pub fn register_arc(&mut self, tool: Arc<dyn Tool>) -> &mut Self {
        let name = tool.name().to_string();
        info!("Registering tool: {}", name);
        self.tools.insert(name, tool);
        self
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// Check if a tool exists
    pub fn contains(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Get all tool names
    pub fn names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Get the number of registered tools
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Execute a tool by name
    pub async fn execute(&self, name: &str, input: Value) -> ToolResult<ToolOutput> {
        let tool = self
            .get(name)
            .ok_or_else(|| ToolError::NotFound(name.to_string()))?;

        debug!("Executing tool: {}", name);

        // Validate input
        tool.validate(&input)?;

        // Execute
        let start = std::time::Instant::now();
        let result = tool.execute(input).await;
        let duration = start.elapsed().as_millis() as u64;

        match result {
            Ok(mut output) => {
                output.duration_ms = duration;
                debug!("Tool {} completed in {}ms", name, duration);
                Ok(output)
            }
            Err(e) => {
                error!("Tool {} failed: {:?}", name, e);
                Err(e)
            }
        }
    }

    /// Generate schemas for all tools (for API tool definitions)
    pub fn schemas(&self) -> Vec<Value> {
        self.tools
            .values()
            .map(|tool| {
                serde_json::json!({
                    "name": tool.name(),
                    "description": tool.description(),
                    "input_schema": tool.schema()
                })
            })
            .collect()
    }

    /// Get tool definitions for Anthropic API format
    pub fn to_anthropic_tools(&self) -> Vec<crate::anthropic::Tool> {
        self.tools
            .values()
            .map(|tool| crate::anthropic::Tool {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                input_schema: tool.schema(),
            })
            .collect()
    }
}

impl std::fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolRegistry")
            .field("tools", &self.tools.keys().collect::<Vec<_>>())
            .finish()
    }
}

// =============================================================================
// Tool Orchestration System
// =============================================================================

/// Tool call request for orchestration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique call ID
    pub id: String,
    /// Tool name
    pub name: String,
    /// Input parameters
    pub input: Value,
    /// Whether approval is required
    #[serde(default)]
    pub requires_approval: bool,
    /// Priority (higher = more urgent)
    #[serde(default)]
    pub priority: i32,
    /// Dependencies (other call IDs that must complete first)
    #[serde(default)]
    pub dependencies: Vec<String>,
}

impl ToolCall {
    /// Create a new tool call
    pub fn new(name: impl Into<String>, input: Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            input,
            requires_approval: false,
            priority: 0,
            dependencies: Vec::new(),
        }
    }

    /// Set approval requirement
    pub fn with_approval(mut self, required: bool) -> Self {
        self.requires_approval = required;
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Add dependency
    pub fn depends_on(mut self, call_id: impl Into<String>) -> Self {
        self.dependencies.push(call_id.into());
        self
    }
}

/// Status of a tool execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    /// Waiting for dependencies
    Pending,
    /// Waiting for approval
    AwaitingApproval,
    /// Currently executing
    Running,
    /// Completed successfully
    Completed,
    /// Failed with error
    Failed,
    /// Cancelled by user
    Cancelled,
    /// Timed out
    TimedOut,
}

/// Record of a single tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    /// Call ID
    pub call_id: String,
    /// Tool name
    pub tool_name: String,
    /// Input parameters
    pub input: Value,
    /// Execution status
    pub status: ExecutionStatus,
    /// Output (if completed)
    pub output: Option<ToolOutput>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// When execution started
    pub started_at: Option<DateTime<Utc>>,
    /// When execution completed
    pub completed_at: Option<DateTime<Utc>>,
    /// Duration in milliseconds
    pub duration_ms: Option<u64>,
}

impl ExecutionRecord {
    /// Create a new pending record
    pub fn new(call: &ToolCall) -> Self {
        Self {
            call_id: call.id.clone(),
            tool_name: call.name.clone(),
            input: call.input.clone(),
            status: ExecutionStatus::Pending,
            output: None,
            error: None,
            started_at: None,
            completed_at: None,
            duration_ms: None,
        }
    }

    /// Mark as running
    pub fn start(&mut self) {
        self.status = ExecutionStatus::Running;
        self.started_at = Some(Utc::now());
    }

    /// Mark as completed
    pub fn complete(&mut self, output: ToolOutput) {
        self.status = ExecutionStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.duration_ms = output.duration_ms.into();
        self.output = Some(output);
    }

    /// Mark as failed
    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = ExecutionStatus::Failed;
        self.completed_at = Some(Utc::now());
        self.error = Some(error.into());
        if let (Some(start), Some(end)) = (self.started_at, self.completed_at) {
            self.duration_ms = Some((end - start).num_milliseconds() as u64);
        }
    }
}

/// Execution history for tracking tool calls
#[derive(Debug, Clone, Default)]
pub struct ExecutionHistory {
    /// All execution records
    records: VecDeque<ExecutionRecord>,
    /// Maximum history size
    max_size: usize,
}

impl ExecutionHistory {
    /// Create new history with default max size
    pub fn new() -> Self {
        Self {
            records: VecDeque::new(),
            max_size: 1000,
        }
    }

    /// Create with custom max size
    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            records: VecDeque::new(),
            max_size,
        }
    }

    /// Add a record
    pub fn add(&mut self, record: ExecutionRecord) {
        if self.records.len() >= self.max_size {
            self.records.pop_front();
        }
        self.records.push_back(record);
    }

    /// Get record by call ID
    pub fn get(&self, call_id: &str) -> Option<&ExecutionRecord> {
        self.records.iter().find(|r| r.call_id == call_id)
    }

    /// Get mutable record by call ID
    pub fn get_mut(&mut self, call_id: &str) -> Option<&mut ExecutionRecord> {
        self.records.iter_mut().find(|r| r.call_id == call_id)
    }

    /// Get all records
    pub fn all(&self) -> impl Iterator<Item = &ExecutionRecord> {
        self.records.iter()
    }

    /// Get recent records (last N)
    pub fn recent(&self, count: usize) -> impl Iterator<Item = &ExecutionRecord> {
        self.records.iter().rev().take(count)
    }

    /// Get records by status
    pub fn by_status(&self, status: ExecutionStatus) -> Vec<&ExecutionRecord> {
        self.records.iter().filter(|r| r.status == status).collect()
    }

    /// Get records for a tool
    pub fn by_tool(&self, tool_name: &str) -> Vec<&ExecutionRecord> {
        self.records
            .iter()
            .filter(|r| r.tool_name == tool_name)
            .collect()
    }

    /// Total execution count
    pub fn total_count(&self) -> usize {
        self.records.len()
    }

    /// Success count
    pub fn success_count(&self) -> usize {
        self.records
            .iter()
            .filter(|r| r.status == ExecutionStatus::Completed)
            .count()
    }

    /// Failure count
    pub fn failure_count(&self) -> usize {
        self.records
            .iter()
            .filter(|r| r.status == ExecutionStatus::Failed)
            .count()
    }

    /// Average execution time (for completed executions)
    pub fn average_duration_ms(&self) -> Option<f64> {
        let durations: Vec<u64> = self.records.iter().filter_map(|r| r.duration_ms).collect();

        if durations.is_empty() {
            None
        } else {
            Some(durations.iter().sum::<u64>() as f64 / durations.len() as f64)
        }
    }

    /// Clear history
    pub fn clear(&mut self) {
        self.records.clear();
    }
}

/// Event emitted during tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionEvent {
    /// Tool call queued
    Queued { call_id: String, tool_name: String },
    /// Waiting for approval
    AwaitingApproval { call_id: String, tool_name: String },
    /// Approval granted
    Approved { call_id: String },
    /// Approval denied
    Denied { call_id: String, reason: String },
    /// Execution started
    Started { call_id: String },
    /// Progress update
    Progress {
        call_id: String,
        progress: f32,
        message: String,
    },
    /// Output chunk (for streaming)
    OutputChunk { call_id: String, chunk: String },
    /// Execution completed
    Completed { call_id: String, output: ToolOutput },
    /// Execution failed
    Failed { call_id: String, error: String },
    /// Execution cancelled
    Cancelled { call_id: String },
}

/// Tool executor for orchestrating multiple tool calls
pub struct ToolExecutor {
    /// Tool registry
    registry: Arc<ToolRegistry>,
    /// Execution history
    history: Arc<RwLock<ExecutionHistory>>,
    /// Event sender
    event_tx: mpsc::Sender<ExecutionEvent>,
    /// Concurrency limit
    max_concurrent: usize,
    /// Default timeout in milliseconds
    default_timeout_ms: u64,
}

impl ToolExecutor {
    /// Create a new executor
    pub fn new(registry: Arc<ToolRegistry>, event_tx: mpsc::Sender<ExecutionEvent>) -> Self {
        Self {
            registry,
            history: Arc::new(RwLock::new(ExecutionHistory::new())),
            event_tx,
            max_concurrent: 4,
            default_timeout_ms: 120_000,
        }
    }

    /// Set concurrency limit
    pub fn with_concurrency(mut self, max: usize) -> Self {
        self.max_concurrent = max.max(1);
        self
    }

    /// Set default timeout
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.default_timeout_ms = timeout_ms;
        self
    }

    /// Execute a single tool call
    pub async fn execute(&self, call: ToolCall) -> ToolResult<ToolOutput> {
        let call_id = call.id.clone();
        let tool_name = call.name.clone();

        // Create record
        let record = ExecutionRecord::new(&call);
        self.history.write().await.add(record);

        // Emit queued event
        let _ = self
            .event_tx
            .send(ExecutionEvent::Queued {
                call_id: call_id.clone(),
                tool_name: tool_name.clone(),
            })
            .await;

        // Check approval requirement
        if call.requires_approval {
            let _ = self
                .event_tx
                .send(ExecutionEvent::AwaitingApproval {
                    call_id: call_id.clone(),
                    tool_name: tool_name.clone(),
                })
                .await;

            if let Some(record) = self.history.write().await.get_mut(&call_id) {
                record.status = ExecutionStatus::AwaitingApproval;
            }

            // In a real implementation, we would wait for approval here
            // For now, we auto-approve
            let _ = self
                .event_tx
                .send(ExecutionEvent::Approved {
                    call_id: call_id.clone(),
                })
                .await;
        }

        // Mark as running
        if let Some(record) = self.history.write().await.get_mut(&call_id) {
            record.start();
        }

        let _ = self
            .event_tx
            .send(ExecutionEvent::Started {
                call_id: call_id.clone(),
            })
            .await;

        // Execute with timeout
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(self.default_timeout_ms),
            self.registry.execute(&call.name, call.input),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                // Success
                if let Some(record) = self.history.write().await.get_mut(&call_id) {
                    record.complete(output.clone());
                }

                let _ = self
                    .event_tx
                    .send(ExecutionEvent::Completed {
                        call_id,
                        output: output.clone(),
                    })
                    .await;

                Ok(output)
            }
            Ok(Err(e)) => {
                // Tool error
                let error_msg = e.to_string();
                if let Some(record) = self.history.write().await.get_mut(&call_id) {
                    record.fail(&error_msg);
                }

                let _ = self
                    .event_tx
                    .send(ExecutionEvent::Failed {
                        call_id,
                        error: error_msg,
                    })
                    .await;

                Err(e)
            }
            Err(_) => {
                // Timeout
                let error_msg = format!("Timeout after {}ms", self.default_timeout_ms);
                if let Some(record) = self.history.write().await.get_mut(&call_id) {
                    record.status = ExecutionStatus::TimedOut;
                    record.error = Some(error_msg.clone());
                    record.completed_at = Some(Utc::now());
                }

                let _ = self
                    .event_tx
                    .send(ExecutionEvent::Failed {
                        call_id,
                        error: error_msg.clone(),
                    })
                    .await;

                Err(ToolError::Timeout(self.default_timeout_ms))
            }
        }
    }

    /// Execute multiple tool calls in parallel
    pub async fn execute_parallel(&self, calls: Vec<ToolCall>) -> Vec<ToolResult<ToolOutput>> {
        use futures::stream::{self, StreamExt};

        let results = stream::iter(calls)
            .map(|call| {
                let executor = self.clone_inner();
                async move { executor.execute(call).await }
            })
            .buffer_unordered(self.max_concurrent)
            .collect::<Vec<_>>()
            .await;

        results
    }

    /// Execute calls respecting dependencies (DAG execution)
    pub async fn execute_dag(
        &self,
        calls: Vec<ToolCall>,
    ) -> HashMap<String, ToolResult<ToolOutput>> {
        let mut results: HashMap<String, ToolResult<ToolOutput>> = HashMap::new();
        let mut completed: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut remaining: Vec<ToolCall> = calls;

        while !remaining.is_empty() {
            // Find calls with satisfied dependencies
            let (ready, not_ready): (Vec<_>, Vec<_>) = remaining
                .into_iter()
                .partition(|call| call.dependencies.iter().all(|dep| completed.contains(dep)));

            if ready.is_empty() && !not_ready.is_empty() {
                // Circular dependency or missing dependency
                warn!("Circular or missing dependencies detected");
                for call in not_ready {
                    results.insert(
                        call.id.clone(),
                        Err(ToolError::ExecutionFailed(
                            "Unresolved dependencies".to_string(),
                        )),
                    );
                }
                break;
            }

            // Execute ready calls in parallel
            let batch_results = self.execute_parallel(ready.clone()).await;

            for (call, result) in ready.into_iter().zip(batch_results) {
                completed.insert(call.id.clone());
                results.insert(call.id, result);
            }

            remaining = not_ready;
        }

        results
    }

    /// Get execution history
    pub async fn history(&self) -> ExecutionHistory {
        self.history.read().await.clone()
    }

    /// Clear history
    pub async fn clear_history(&self) {
        self.history.write().await.clear();
    }

    /// Clone inner state for async operations
    fn clone_inner(&self) -> Self {
        Self {
            registry: self.registry.clone(),
            history: self.history.clone(),
            event_tx: self.event_tx.clone(),
            max_concurrent: self.max_concurrent,
            default_timeout_ms: self.default_timeout_ms,
        }
    }
}

/// Builder for creating execution plans
#[derive(Debug, Clone, Default)]
pub struct ExecutionPlan {
    /// Tool calls in the plan
    calls: Vec<ToolCall>,
}

impl ExecutionPlan {
    /// Create a new empty plan
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a tool call
    pub fn add(&mut self, call: ToolCall) -> &mut Self {
        self.calls.push(call);
        self
    }

    /// Add a tool call (builder style)
    pub fn with_call(mut self, call: ToolCall) -> Self {
        self.calls.push(call);
        self
    }

    /// Get all calls
    pub fn calls(&self) -> &[ToolCall] {
        &self.calls
    }

    /// Take calls for execution
    pub fn take_calls(self) -> Vec<ToolCall> {
        self.calls
    }

    /// Number of calls
    pub fn len(&self) -> usize {
        self.calls.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.calls.is_empty()
    }

    /// Sort calls by priority (highest first)
    pub fn sort_by_priority(&mut self) {
        self.calls
            .sort_by_key(|call| std::cmp::Reverse(call.priority));
    }
}

/// Statistics about tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStats {
    /// Total executions
    pub total: usize,
    /// Successful executions
    pub successful: usize,
    /// Failed executions
    pub failed: usize,
    /// Average duration in milliseconds
    pub avg_duration_ms: Option<f64>,
    /// Most used tools
    pub tool_usage: HashMap<String, usize>,
}

impl ExecutionStats {
    /// Calculate stats from history
    pub fn from_history(history: &ExecutionHistory) -> Self {
        let mut tool_usage: HashMap<String, usize> = HashMap::new();

        for record in history.all() {
            *tool_usage.entry(record.tool_name.clone()).or_insert(0) += 1;
        }

        Self {
            total: history.total_count(),
            successful: history.success_count(),
            failed: history.failure_count(),
            avg_duration_ms: history.average_duration_ms(),
            tool_usage,
        }
    }

    /// Success rate (0.0 - 1.0)
    pub fn success_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.successful as f64 / self.total as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test tool implementation
    struct EchoTool;

    #[async_trait]
    impl Tool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }

        fn description(&self) -> &str {
            "Echoes back the input message"
        }

        fn parameters(&self) -> Vec<ParameterDef> {
            vec![ParameterDef::required_string(
                "message",
                "The message to echo",
            )]
        }

        async fn execute(&self, input: Value) -> ToolResult<ToolOutput> {
            let message = input
                .get("message")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidInput("message is required".to_string()))?;

            Ok(ToolOutput::success(serde_json::json!({
                "echoed": message
            })))
        }
    }

    // Another test tool
    struct AddTool;

    #[async_trait]
    impl Tool for AddTool {
        fn name(&self) -> &str {
            "add"
        }

        fn description(&self) -> &str {
            "Adds two numbers"
        }

        fn parameters(&self) -> Vec<ParameterDef> {
            vec![
                ParameterDef {
                    name: "a".to_string(),
                    param_type: "number".to_string(),
                    description: "First number".to_string(),
                    required: true,
                    default: None,
                    enum_values: None,
                },
                ParameterDef {
                    name: "b".to_string(),
                    param_type: "number".to_string(),
                    description: "Second number".to_string(),
                    required: true,
                    default: None,
                    enum_values: None,
                },
            ]
        }

        async fn execute(&self, input: Value) -> ToolResult<ToolOutput> {
            let a = input
                .get("a")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| ToolError::InvalidInput("a is required".to_string()))?;

            let b = input
                .get("b")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| ToolError::InvalidInput("b is required".to_string()))?;

            Ok(ToolOutput::success(serde_json::json!({
                "result": a + b
            })))
        }
    }

    #[test]
    fn test_registry_creation() {
        let registry = ToolRegistry::new();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_tool_registration() {
        let mut registry = ToolRegistry::new();
        registry.register(EchoTool);

        assert_eq!(registry.len(), 1);
        assert!(registry.contains("echo"));
    }

    #[test]
    fn test_tool_lookup() {
        let mut registry = ToolRegistry::new();
        registry.register(EchoTool);

        let tool = registry.get("echo");
        assert!(tool.is_some());
        assert_eq!(tool.unwrap().name(), "echo");

        let missing = registry.get("nonexistent");
        assert!(missing.is_none());
    }

    #[test]
    fn test_schema_generation() {
        let tool = EchoTool;
        let schema = tool.schema();

        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["message"].is_object());
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .contains(&Value::String("message".to_string())));
    }

    #[tokio::test]
    async fn test_tool_execution() {
        let mut registry = ToolRegistry::new();
        registry.register(EchoTool);

        let result = registry
            .execute("echo", serde_json::json!({ "message": "hello" }))
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.success);
        assert_eq!(output.content["echoed"], "hello");
    }

    #[tokio::test]
    async fn test_tool_not_found() {
        let registry = ToolRegistry::new();

        let result = registry.execute("missing", serde_json::json!({})).await;

        assert!(matches!(result, Err(ToolError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_tool_validation() {
        let mut registry = ToolRegistry::new();
        registry.register(EchoTool);

        // Missing required parameter
        let result = registry.execute("echo", serde_json::json!({})).await;
        assert!(matches!(result, Err(ToolError::ValidationError(_))));
    }

    #[tokio::test]
    async fn test_multiple_tools() {
        let mut registry = ToolRegistry::new();
        registry.register(EchoTool).register(AddTool);

        assert_eq!(registry.len(), 2);

        let names = registry.names();
        assert!(names.contains(&"echo".to_string()));
        assert!(names.contains(&"add".to_string()));
    }

    #[tokio::test]
    async fn test_add_tool() {
        let mut registry = ToolRegistry::new();
        registry.register(AddTool);

        let result = registry
            .execute("add", serde_json::json!({ "a": 5, "b": 3 }))
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.content["result"], 8.0);
    }

    #[test]
    fn test_schemas_generation() {
        let mut registry = ToolRegistry::new();
        registry.register(EchoTool).register(AddTool);

        let schemas = registry.schemas();
        assert_eq!(schemas.len(), 2);
    }

    #[test]
    fn test_parameter_def_builders() {
        let param =
            ParameterDef::required_string("test", "Test parameter").with_default("default_value");

        assert_eq!(param.name, "test");
        assert!(!param.required); // Setting default makes it optional
        assert_eq!(
            param.default,
            Some(Value::String("default_value".to_string()))
        );
    }

    #[test]
    fn test_tool_output() {
        let success = ToolOutput::success("test").with_duration(100);
        assert!(success.success);
        assert_eq!(success.duration_ms, 100);

        let failure = ToolOutput::failure("error");
        assert!(!failure.success);
        assert_eq!(failure.error, Some("error".to_string()));
    }

    #[test]
    fn test_output_format_display_string() {
        let output = ToolOutput::success("Hello, world!");
        assert_eq!(output.format_display(), "Hello, world!");
    }

    #[test]
    fn test_output_format_display_object() {
        let output = ToolOutput::success(serde_json::json!({
            "key": "value"
        }));
        let display = output.format_display();
        assert!(display.contains("key"));
        assert!(display.contains("value"));
    }

    #[test]
    fn test_output_format_display_error() {
        let output = ToolOutput::failure("Something went wrong");
        assert_eq!(output.format_display(), "Something went wrong");
    }

    #[test]
    fn test_output_summary_truncation() {
        let output = ToolOutput::success("This is a very long string that should be truncated");
        let summary = output.summary(20);
        assert_eq!(summary.len(), 20);
        assert!(summary.ends_with("..."));
    }

    #[test]
    fn test_output_summary_no_truncation() {
        let output = ToolOutput::success("Short");
        let summary = output.summary(20);
        assert_eq!(summary, "Short");
    }

    #[test]
    fn test_output_as_text() {
        // String content
        let output = ToolOutput::success("text content");
        assert_eq!(output.as_text(), Some("text content".to_string()));

        // Object with content field
        let output = ToolOutput::success(serde_json::json!({
            "content": "nested text"
        }));
        assert_eq!(output.as_text(), Some("nested text".to_string()));

        // Object with stdout field
        let output = ToolOutput::success(serde_json::json!({
            "stdout": "command output"
        }));
        assert_eq!(output.as_text(), Some("command output".to_string()));

        // Array (no text)
        let output = ToolOutput::success(serde_json::json!([1, 2, 3]));
        assert_eq!(output.as_text(), None);
    }

    #[test]
    fn test_output_format_duration() {
        let output = ToolOutput::success("").with_duration(500);
        assert_eq!(output.format_duration(), "500ms");

        let output = ToolOutput::success("").with_duration(1500);
        assert_eq!(output.format_duration(), "1.50s");

        let output = ToolOutput::success("").with_duration(60000);
        assert_eq!(output.format_duration(), "60.00s");
    }

    #[test]
    fn test_output_status_indicator() {
        let success = ToolOutput::success("");
        assert_eq!(success.status_indicator(), "✓");

        let failure = ToolOutput::failure("error");
        assert_eq!(failure.status_indicator(), "✗");
    }

    #[test]
    fn test_output_has_error_code() {
        let output = ToolOutput::success(serde_json::json!({
            "exit_code": 0
        }));
        assert!(!output.has_error_code());

        let output = ToolOutput::success(serde_json::json!({
            "exit_code": 1
        }));
        assert!(output.has_error_code());

        let output = ToolOutput::success("no exit code");
        assert!(!output.has_error_code());
    }
}
