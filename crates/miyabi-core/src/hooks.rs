//! Hooks System for Event-Driven Execution
//!
//! Provides a system for registering and executing hooks based on events.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Hook event types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    /// Before sending a message
    PreMessage,
    /// After receiving a response
    PostMessage,
    /// Before executing a tool
    PreTool,
    /// After tool execution
    PostTool,
    /// Before committing changes
    PreCommit,
    /// After committing changes
    PostCommit,
    /// On session start
    SessionStart,
    /// On session end
    SessionEnd,
    /// On error
    OnError,
    /// Custom event
    Custom(String),
}

impl std::fmt::Display for HookEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HookEvent::PreMessage => write!(f, "pre_message"),
            HookEvent::PostMessage => write!(f, "post_message"),
            HookEvent::PreTool => write!(f, "pre_tool"),
            HookEvent::PostTool => write!(f, "post_tool"),
            HookEvent::PreCommit => write!(f, "pre_commit"),
            HookEvent::PostCommit => write!(f, "post_commit"),
            HookEvent::SessionStart => write!(f, "session_start"),
            HookEvent::SessionEnd => write!(f, "session_end"),
            HookEvent::OnError => write!(f, "on_error"),
            HookEvent::Custom(name) => write!(f, "custom:{}", name),
        }
    }
}

/// Hook action types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HookAction {
    /// Run a shell command
    Shell { command: String },
    /// Run an agent with a prompt
    Agent {
        prompt: String,
        #[serde(default)]
        spec: Option<String>,
    },
    /// Send a notification
    Notify { title: String, message: String },
    /// Log a message
    Log { level: String, message: String },
    /// Execute a script file
    Script { path: PathBuf },
}

/// A single hook definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hook {
    /// Hook name for identification
    pub name: String,
    /// Event that triggers this hook
    pub event: HookEvent,
    /// Action to perform
    pub action: HookAction,
    /// Whether the hook is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Optional condition (tool name, etc.)
    #[serde(default)]
    pub condition: Option<String>,
    /// Timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

fn default_true() -> bool {
    true
}

fn default_timeout() -> u64 {
    30
}

/// Hook execution context
#[derive(Debug, Clone, Default)]
pub struct HookContext {
    /// Event-specific data
    pub data: HashMap<String, String>,
    /// Tool name (for tool events)
    pub tool_name: Option<String>,
    /// Tool result (for post_tool)
    pub tool_result: Option<String>,
    /// Error message (for on_error)
    pub error: Option<String>,
}

impl HookContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_tool(mut self, name: &str) -> Self {
        self.tool_name = Some(name.to_string());
        self
    }

    pub fn with_result(mut self, result: &str) -> Self {
        self.tool_result = Some(result.to_string());
        self
    }

    pub fn with_error(mut self, error: &str) -> Self {
        self.error = Some(error.to_string());
        self
    }

    pub fn with_data(mut self, key: &str, value: &str) -> Self {
        self.data.insert(key.to_string(), value.to_string());
        self
    }
}

/// Hook execution result
#[derive(Debug)]
pub struct HookResult {
    pub hook_name: String,
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
}

/// Hooks configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HooksConfig {
    /// List of hooks
    #[serde(default)]
    pub hooks: Vec<Hook>,
}

impl HooksConfig {
    /// Load hooks from a YAML file
    pub fn load(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: HooksConfig = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    /// Load hooks from default location (~/.miyabi/hooks.yml)
    pub fn load_default() -> Result<Self> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let path = home.join(".miyabi").join("hooks.yml");
        if path.exists() {
            Self::load(&path)
        } else {
            Ok(Self::default())
        }
    }

    /// Save hooks to a file
    pub fn save(&self, path: &PathBuf) -> Result<()> {
        let content = serde_yaml::to_string(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

/// Hook manager for registering and executing hooks
pub struct HookManager {
    hooks: Vec<Hook>,
}

impl Default for HookManager {
    fn default() -> Self {
        Self::new()
    }
}

impl HookManager {
    /// Create a new hook manager
    pub fn new() -> Self {
        Self { hooks: Vec::new() }
    }

    /// Create from config
    pub fn from_config(config: HooksConfig) -> Self {
        Self {
            hooks: config.hooks,
        }
    }

    /// Load hooks from default location
    pub fn load_default() -> Result<Self> {
        let config = HooksConfig::load_default()?;
        Ok(Self::from_config(config))
    }

    /// Register a hook
    pub fn register(&mut self, hook: Hook) {
        self.hooks.push(hook);
    }

    /// Get hooks for an event
    pub fn get_hooks(&self, event: &HookEvent) -> Vec<&Hook> {
        self.hooks
            .iter()
            .filter(|h| h.enabled && &h.event == event)
            .collect()
    }

    /// Execute hooks for an event
    pub async fn execute(&self, event: &HookEvent, context: &HookContext) -> Vec<HookResult> {
        let hooks = self.get_hooks(event);
        let mut results = Vec::new();

        for hook in hooks {
            // Check condition
            if let Some(condition) = &hook.condition {
                if let Some(tool_name) = &context.tool_name {
                    if condition != tool_name {
                        continue;
                    }
                }
            }

            let result = self.execute_hook(hook, context).await;
            results.push(result);
        }

        results
    }

    /// Execute a single hook
    async fn execute_hook(&self, hook: &Hook, context: &HookContext) -> HookResult {
        match &hook.action {
            HookAction::Shell { command } => {
                let expanded = self.expand_variables(command, context);
                match self.run_shell_command(&expanded, hook.timeout).await {
                    Ok(output) => HookResult {
                        hook_name: hook.name.clone(),
                        success: true,
                        output: Some(output),
                        error: None,
                    },
                    Err(e) => HookResult {
                        hook_name: hook.name.clone(),
                        success: false,
                        output: None,
                        error: Some(e.to_string()),
                    },
                }
            }
            HookAction::Notify { title, message } => {
                let expanded_title = self.expand_variables(title, context);
                let expanded_message = self.expand_variables(message, context);
                // For now, just log the notification
                tracing::info!("Hook notification: {} - {}", expanded_title, expanded_message);
                HookResult {
                    hook_name: hook.name.clone(),
                    success: true,
                    output: Some(format!("{}: {}", expanded_title, expanded_message)),
                    error: None,
                }
            }
            HookAction::Log { level, message } => {
                let expanded = self.expand_variables(message, context);
                match level.as_str() {
                    "error" => tracing::error!("Hook: {}", expanded),
                    "warn" => tracing::warn!("Hook: {}", expanded),
                    "info" => tracing::info!("Hook: {}", expanded),
                    "debug" => tracing::debug!("Hook: {}", expanded),
                    _ => tracing::info!("Hook: {}", expanded),
                }
                HookResult {
                    hook_name: hook.name.clone(),
                    success: true,
                    output: Some(expanded),
                    error: None,
                }
            }
            HookAction::Script { path } => {
                if !path.exists() {
                    return HookResult {
                        hook_name: hook.name.clone(),
                        success: false,
                        output: None,
                        error: Some(format!("Script not found: {:?}", path)),
                    };
                }
                let command = path.to_string_lossy().to_string();
                match self.run_shell_command(&command, hook.timeout).await {
                    Ok(output) => HookResult {
                        hook_name: hook.name.clone(),
                        success: true,
                        output: Some(output),
                        error: None,
                    },
                    Err(e) => HookResult {
                        hook_name: hook.name.clone(),
                        success: false,
                        output: None,
                        error: Some(e.to_string()),
                    },
                }
            }
            HookAction::Agent { prompt, spec: _ } => {
                // Agent execution would require the full agent infrastructure
                // For now, return a placeholder
                let expanded = self.expand_variables(prompt, context);
                HookResult {
                    hook_name: hook.name.clone(),
                    success: true,
                    output: Some(format!("Agent prompt: {}", expanded)),
                    error: None,
                }
            }
        }
    }

    /// Expand variables in a string
    fn expand_variables(&self, input: &str, context: &HookContext) -> String {
        let mut result = input.to_string();

        // Replace context variables
        if let Some(tool_name) = &context.tool_name {
            result = result.replace("${tool_name}", tool_name);
        }
        if let Some(tool_result) = &context.tool_result {
            result = result.replace("${tool_result}", tool_result);
        }
        if let Some(error) = &context.error {
            result = result.replace("${error}", error);
        }

        // Replace custom data
        for (key, value) in &context.data {
            result = result.replace(&format!("${{{}}}", key), value);
        }

        result
    }

    /// Run a shell command
    async fn run_shell_command(&self, command: &str, timeout_secs: u64) -> Result<String> {
        use std::process::Stdio;
        use tokio::process::Command;
        use tokio::time::{timeout, Duration};

        let output = timeout(
            Duration::from_secs(timeout_secs),
            Command::new("sh")
                .arg("-c")
                .arg(command)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output(),
        )
        .await??;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("Command failed: {}", stderr))
        }
    }

    /// Get all registered hooks
    pub fn list_hooks(&self) -> &[Hook] {
        &self.hooks
    }

    /// Enable a hook by name
    pub fn enable(&mut self, name: &str) {
        for hook in &mut self.hooks {
            if hook.name == name {
                hook.enabled = true;
            }
        }
    }

    /// Disable a hook by name
    pub fn disable(&mut self, name: &str) {
        for hook in &mut self.hooks {
            if hook.name == name {
                hook.enabled = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_event_display() {
        assert_eq!(HookEvent::PreMessage.to_string(), "pre_message");
        assert_eq!(HookEvent::PostTool.to_string(), "post_tool");
        assert_eq!(
            HookEvent::Custom("test".to_string()).to_string(),
            "custom:test"
        );
    }

    #[test]
    fn test_hook_context() {
        let context = HookContext::new()
            .with_tool("bash")
            .with_result("success")
            .with_data("key", "value");

        assert_eq!(context.tool_name, Some("bash".to_string()));
        assert_eq!(context.tool_result, Some("success".to_string()));
        assert_eq!(context.data.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_expand_variables() {
        let manager = HookManager::new();
        let context = HookContext::new()
            .with_tool("bash")
            .with_result("ok")
            .with_data("name", "test");

        let input = "Tool: ${tool_name}, Result: ${tool_result}, Name: ${name}";
        let expanded = manager.expand_variables(input, &context);
        assert_eq!(expanded, "Tool: bash, Result: ok, Name: test");
    }

    #[test]
    fn test_hook_config_serialization() {
        let config = HooksConfig {
            hooks: vec![Hook {
                name: "test".to_string(),
                event: HookEvent::PostTool,
                action: HookAction::Log {
                    level: "info".to_string(),
                    message: "Tool completed".to_string(),
                },
                enabled: true,
                condition: Some("bash".to_string()),
                timeout: 30,
            }],
        };

        let yaml = serde_yaml::to_string(&config).unwrap();
        assert!(yaml.contains("post_tool"));
        assert!(yaml.contains("bash"));
    }

    #[test]
    fn test_hook_manager_get_hooks() {
        let mut manager = HookManager::new();
        manager.register(Hook {
            name: "hook1".to_string(),
            event: HookEvent::PreTool,
            action: HookAction::Log {
                level: "info".to_string(),
                message: "test".to_string(),
            },
            enabled: true,
            condition: None,
            timeout: 30,
        });
        manager.register(Hook {
            name: "hook2".to_string(),
            event: HookEvent::PostTool,
            action: HookAction::Log {
                level: "info".to_string(),
                message: "test".to_string(),
            },
            enabled: true,
            condition: None,
            timeout: 30,
        });

        let pre_hooks = manager.get_hooks(&HookEvent::PreTool);
        assert_eq!(pre_hooks.len(), 1);
        assert_eq!(pre_hooks[0].name, "hook1");

        let post_hooks = manager.get_hooks(&HookEvent::PostTool);
        assert_eq!(post_hooks.len(), 1);
        assert_eq!(post_hooks[0].name, "hook2");
    }
}
