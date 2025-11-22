//! Tool Trait and Registry System
//!
//! This module provides the core abstractions for defining and managing
//! tools that can be executed by AI agents.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, error, info};

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
            prop.insert("description".to_string(), Value::String(param.description.clone()));

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
            if param.required {
                if input.get(&param.name).is_none() {
                    return Err(ToolError::ValidationError(format!(
                        "Required parameter '{}' is missing",
                        param.name
                    )));
                }
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
        assert!(schema["required"].as_array().unwrap().contains(&Value::String("message".to_string())));
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

        let result = registry
            .execute("missing", serde_json::json!({}))
            .await;

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
        let param = ParameterDef::required_string("test", "Test parameter")
            .with_default("default_value");

        assert_eq!(param.name, "test");
        assert!(!param.required); // Setting default makes it optional
        assert_eq!(param.default, Some(Value::String("default_value".to_string())));
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
}
