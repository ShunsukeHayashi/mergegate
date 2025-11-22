//! MCP (Model Context Protocol) Support
//!
//! Integration with external MCP tool servers.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Server name
    pub name: String,
    /// Command to start the server
    pub command: String,
    /// Command arguments
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Whether to auto-start
    #[serde(default = "default_true")]
    pub auto_start: bool,
}

fn default_true() -> bool {
    true
}

/// MCP servers configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpConfig {
    /// List of server configurations
    #[serde(default)]
    pub servers: Vec<McpServerConfig>,
}

impl McpConfig {
    /// Load configuration from a YAML file
    pub fn load(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: McpConfig = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    /// Load from default location (~/.miyabi/mcp.yml)
    pub fn load_default() -> Result<Self> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let path = home.join(".miyabi").join("mcp.yml");
        if path.exists() {
            Self::load(&path)
        } else {
            Ok(Self::default())
        }
    }

    /// Save configuration to a file
    pub fn save(&self, path: &PathBuf) -> Result<()> {
        let content = serde_yaml::to_string(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

/// MCP JSON-RPC request
#[derive(Debug, Clone, Serialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// MCP JSON-RPC response
#[derive(Debug, Clone, Deserialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<McpError>,
}

/// MCP error
#[derive(Debug, Clone, Deserialize)]
pub struct McpError {
    pub code: i32,
    pub message: String,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

/// MCP tool definition
#[derive(Debug, Clone, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub input_schema: Option<serde_json::Value>,
}

/// MCP server connection
pub struct McpServer {
    pub name: String,
    process: Child,
    request_id: u64,
}

impl McpServer {
    /// Start a new MCP server
    pub async fn start(config: &McpServerConfig) -> Result<Self> {
        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        let process = cmd.spawn()?;

        Ok(Self {
            name: config.name.clone(),
            process,
            request_id: 0,
        })
    }

    /// Send a request and get a response
    pub async fn request(&mut self, method: &str, params: Option<serde_json::Value>) -> Result<McpResponse> {
        self.request_id += 1;
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: self.request_id,
            method: method.to_string(),
            params,
        };

        let stdin = self.process.stdin.as_mut()
            .ok_or_else(|| anyhow::anyhow!("Failed to get stdin"))?;
        let stdout = self.process.stdout.as_mut()
            .ok_or_else(|| anyhow::anyhow!("Failed to get stdout"))?;

        // Send request
        let request_json = serde_json::to_string(&request)?;
        stdin.write_all(request_json.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;

        // Read response
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader.read_line(&mut line).await?;

        let response: McpResponse = serde_json::from_str(&line)?;
        Ok(response)
    }

    /// Initialize the server
    pub async fn initialize(&mut self) -> Result<serde_json::Value> {
        let params = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "clientInfo": {
                "name": "miyabi",
                "version": env!("CARGO_PKG_VERSION")
            }
        });

        let response = self.request("initialize", Some(params)).await?;
        if let Some(error) = response.error {
            return Err(anyhow::anyhow!("MCP error: {}", error.message));
        }

        response.result.ok_or_else(|| anyhow::anyhow!("No result in response"))
    }

    /// List available tools
    pub async fn list_tools(&mut self) -> Result<Vec<McpTool>> {
        let response = self.request("tools/list", None).await?;
        if let Some(error) = response.error {
            return Err(anyhow::anyhow!("MCP error: {}", error.message));
        }

        let result = response.result.ok_or_else(|| anyhow::anyhow!("No result"))?;
        let tools: Vec<McpTool> = serde_json::from_value(
            result.get("tools").cloned().unwrap_or(serde_json::Value::Array(vec![]))
        )?;

        Ok(tools)
    }

    /// Call a tool
    pub async fn call_tool(&mut self, name: &str, arguments: serde_json::Value) -> Result<serde_json::Value> {
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments
        });

        let response = self.request("tools/call", Some(params)).await?;
        if let Some(error) = response.error {
            return Err(anyhow::anyhow!("MCP error: {}", error.message));
        }

        response.result.ok_or_else(|| anyhow::anyhow!("No result"))
    }

    /// Shutdown the server
    pub async fn shutdown(&mut self) -> Result<()> {
        let _ = self.request("shutdown", None).await;
        self.process.kill().await?;
        Ok(())
    }
}

/// MCP Manager for handling multiple servers
pub struct McpManager {
    servers: HashMap<String, McpServer>,
    config: McpConfig,
}

impl McpManager {
    /// Create a new MCP manager
    pub fn new() -> Self {
        Self {
            servers: HashMap::new(),
            config: McpConfig::default(),
        }
    }

    /// Create from configuration
    pub fn with_config(config: McpConfig) -> Self {
        Self {
            servers: HashMap::new(),
            config,
        }
    }

    /// Load configuration from default location
    pub fn load_default() -> Result<Self> {
        let config = McpConfig::load_default()?;
        Ok(Self::with_config(config))
    }

    /// Start all configured servers
    pub async fn start_all(&mut self) -> Result<()> {
        let names: Vec<_> = self
            .config
            .servers
            .iter()
            .filter(|s| s.auto_start)
            .map(|s| s.name.clone())
            .collect();

        for name in names {
            self.start_server(&name).await?;
        }
        Ok(())
    }

    /// Start a specific server
    pub async fn start_server(&mut self, name: &str) -> Result<()> {
        let config = self
            .config
            .servers
            .iter()
            .find(|s| s.name == name)
            .ok_or_else(|| anyhow::anyhow!("Server not found: {}", name))?
            .clone();

        let mut server = McpServer::start(&config).await?;
        server.initialize().await?;
        self.servers.insert(name.to_string(), server);
        Ok(())
    }

    /// Stop a specific server
    pub async fn stop_server(&mut self, name: &str) -> Result<()> {
        if let Some(mut server) = self.servers.remove(name) {
            server.shutdown().await?;
        }
        Ok(())
    }

    /// Stop all servers
    pub async fn stop_all(&mut self) -> Result<()> {
        let names: Vec<_> = self.servers.keys().cloned().collect();
        for name in names {
            self.stop_server(&name).await?;
        }
        Ok(())
    }

    /// Get a server by name
    pub fn get_server(&mut self, name: &str) -> Option<&mut McpServer> {
        self.servers.get_mut(name)
    }

    /// List all available tools across all servers
    pub async fn list_all_tools(&mut self) -> Result<HashMap<String, Vec<McpTool>>> {
        let mut all_tools = HashMap::new();
        let names: Vec<_> = self.servers.keys().cloned().collect();

        for name in names {
            if let Some(server) = self.servers.get_mut(&name) {
                let tools = server.list_tools().await?;
                all_tools.insert(name, tools);
            }
        }

        Ok(all_tools)
    }

    /// Call a tool on a specific server
    pub async fn call_tool(
        &mut self,
        server_name: &str,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let server = self
            .get_server(server_name)
            .ok_or_else(|| anyhow::anyhow!("Server not found: {}", server_name))?;

        server.call_tool(tool_name, arguments).await
    }

    /// List running servers
    pub fn list_servers(&self) -> Vec<&str> {
        self.servers.keys().map(|s| s.as_str()).collect()
    }

    /// Check if a server is running
    pub fn is_running(&self, name: &str) -> bool {
        self.servers.contains_key(name)
    }
}

impl Default for McpManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_config_serialization() {
        let config = McpConfig {
            servers: vec![McpServerConfig {
                name: "filesystem".to_string(),
                command: "npx".to_string(),
                args: vec!["@anthropic/mcp-server-filesystem".to_string()],
                env: HashMap::new(),
                auto_start: true,
            }],
        };

        let yaml = serde_yaml::to_string(&config).unwrap();
        assert!(yaml.contains("filesystem"));
        assert!(yaml.contains("npx"));
    }

    #[test]
    fn test_mcp_request_serialization() {
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "tools/list".to_string(),
            params: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("tools/list"));
        assert!(!json.contains("params"));
    }

    #[test]
    fn test_mcp_manager() {
        let config = McpConfig {
            servers: vec![McpServerConfig {
                name: "test".to_string(),
                command: "echo".to_string(),
                args: vec![],
                env: HashMap::new(),
                auto_start: false,
            }],
        };

        let manager = McpManager::with_config(config);
        assert!(!manager.is_running("test"));
        assert_eq!(manager.list_servers().len(), 0);
    }

    #[test]
    fn test_mcp_server_config_defaults() {
        let yaml = r#"
name: test-server
command: node
"#;
        let config: McpServerConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.name, "test-server");
        assert_eq!(config.command, "node");
        assert!(config.args.is_empty());
        assert!(config.env.is_empty());
        assert!(config.auto_start); // default_true
    }

    #[test]
    fn test_mcp_server_config_full() {
        let config = McpServerConfig {
            name: "filesystem".to_string(),
            command: "npx".to_string(),
            args: vec!["-y".to_string(), "@anthropic/mcp-server-filesystem".to_string()],
            env: {
                let mut env = HashMap::new();
                env.insert("HOME".to_string(), "/tmp".to_string());
                env
            },
            auto_start: false,
        };

        let yaml = serde_yaml::to_string(&config).unwrap();
        assert!(yaml.contains("filesystem"));
        assert!(yaml.contains("npx"));
        assert!(yaml.contains("-y"));
        assert!(yaml.contains("HOME"));
        assert!(yaml.contains("auto_start: false"));
    }

    #[test]
    fn test_mcp_config_default() {
        let config = McpConfig::default();
        assert!(config.servers.is_empty());
    }

    #[test]
    fn test_mcp_config_multiple_servers() {
        let config = McpConfig {
            servers: vec![
                McpServerConfig {
                    name: "server1".to_string(),
                    command: "cmd1".to_string(),
                    args: vec![],
                    env: HashMap::new(),
                    auto_start: true,
                },
                McpServerConfig {
                    name: "server2".to_string(),
                    command: "cmd2".to_string(),
                    args: vec!["arg1".to_string()],
                    env: HashMap::new(),
                    auto_start: false,
                },
            ],
        };

        assert_eq!(config.servers.len(), 2);
        assert_eq!(config.servers[0].name, "server1");
        assert_eq!(config.servers[1].name, "server2");
    }

    #[test]
    fn test_mcp_config_save_and_load() {
        use tempfile::TempDir;

        let config = McpConfig {
            servers: vec![McpServerConfig {
                name: "test".to_string(),
                command: "test-cmd".to_string(),
                args: vec!["--flag".to_string()],
                env: HashMap::new(),
                auto_start: true,
            }],
        };

        let dir = TempDir::new().unwrap();
        let path = dir.path().join("mcp.yml");

        config.save(&path).unwrap();
        assert!(path.exists());

        let loaded = McpConfig::load(&path).unwrap();
        assert_eq!(loaded.servers.len(), 1);
        assert_eq!(loaded.servers[0].name, "test");
        assert_eq!(loaded.servers[0].command, "test-cmd");
    }

    #[test]
    fn test_mcp_request_with_params() {
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: 42,
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": "read_file",
                "arguments": {
                    "path": "/tmp/test.txt"
                }
            })),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("tools/call"));
        assert!(json.contains("read_file"));
        assert!(json.contains("/tmp/test.txt"));
        assert!(json.contains("42"));
    }

    #[test]
    fn test_mcp_response_success() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "result": {"tools": []}
        }"#;

        let response: McpResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, 1);
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_mcp_response_error() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": 2,
            "error": {
                "code": -32600,
                "message": "Invalid Request"
            }
        }"#;

        let response: McpResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.id, 2);
        assert!(response.result.is_none());
        assert!(response.error.is_some());
        let error = response.error.unwrap();
        assert_eq!(error.code, -32600);
        assert_eq!(error.message, "Invalid Request");
    }

    #[test]
    fn test_mcp_error_with_data() {
        let json = r#"{
            "code": -32602,
            "message": "Invalid params",
            "data": {"expected": "string", "got": "number"}
        }"#;

        let error: McpError = serde_json::from_str(json).unwrap();
        assert_eq!(error.code, -32602);
        assert_eq!(error.message, "Invalid params");
        assert!(error.data.is_some());
    }

    #[test]
    fn test_mcp_tool_deserialization() {
        let json = r#"{
            "name": "read_file",
            "description": "Read contents of a file",
            "input_schema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                },
                "required": ["path"]
            }
        }"#;

        let tool: McpTool = serde_json::from_str(json).unwrap();
        assert_eq!(tool.name, "read_file");
        assert_eq!(tool.description, "Read contents of a file");
        assert!(tool.input_schema.is_some());
    }

    #[test]
    fn test_mcp_tool_minimal() {
        let json = r#"{
            "name": "simple_tool",
            "description": "A simple tool"
        }"#;

        let tool: McpTool = serde_json::from_str(json).unwrap();
        assert_eq!(tool.name, "simple_tool");
        assert!(tool.input_schema.is_none());
    }

    #[test]
    fn test_mcp_manager_new() {
        let manager = McpManager::new();
        assert!(manager.list_servers().is_empty());
    }

    #[test]
    fn test_mcp_manager_default() {
        let manager = McpManager::default();
        assert!(manager.list_servers().is_empty());
    }

    #[test]
    fn test_mcp_manager_is_running() {
        let config = McpConfig {
            servers: vec![
                McpServerConfig {
                    name: "server1".to_string(),
                    command: "cmd".to_string(),
                    args: vec![],
                    env: HashMap::new(),
                    auto_start: false,
                },
            ],
        };

        let manager = McpManager::with_config(config);
        assert!(!manager.is_running("server1"));
        assert!(!manager.is_running("nonexistent"));
    }

    #[test]
    fn test_mcp_config_deserialization() {
        let yaml = r#"
servers:
  - name: filesystem
    command: npx
    args:
      - "-y"
      - "@anthropic/mcp-server-filesystem"
    env:
      ALLOWED_DIRS: /tmp
    auto_start: true
  - name: github
    command: npx
    args:
      - "@anthropic/mcp-server-github"
    auto_start: false
"#;

        let config: McpConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.servers.len(), 2);
        assert_eq!(config.servers[0].name, "filesystem");
        assert!(config.servers[0].auto_start);
        assert_eq!(config.servers[1].name, "github");
        assert!(!config.servers[1].auto_start);
    }

    #[test]
    fn test_default_true_function() {
        assert!(default_true());
    }
}
