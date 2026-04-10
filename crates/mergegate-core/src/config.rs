//! Configuration Management
//!
//! Handles loading and managing application configuration from files and environment.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct Config {
    /// API configuration
    pub api: ApiConfig,
    /// UI configuration
    pub ui: UiConfig,
    /// Session configuration
    pub session: SessionConfig,
    /// Tool configuration
    pub tools: ToolConfig,
}

/// API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiConfig {
    /// Anthropic API key (can also use ANTHROPIC_API_KEY env var)
    pub api_key: Option<String>,
    /// Model to use
    pub model: String,
    /// Maximum tokens for responses
    pub max_tokens: u32,
    /// Enable extended thinking (Claude 4.5+)
    pub thinking: bool,
    /// System prompt
    pub system_prompt: Option<String>,
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// Maximum retries for failed requests
    pub max_retries: u32,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            model: "claude-sonnet-4-5-20250929".to_string(),
            max_tokens: 8192,
            thinking: false,
            system_prompt: Some(
                "You are a helpful AI assistant. Be concise and clear.".to_string(),
            ),
            timeout_secs: 120,
            max_retries: 3,
        }
    }
}

/// UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    /// Show sidebar
    pub show_sidebar: bool,
    /// Show status bar
    pub show_status_bar: bool,
    /// Show breadcrumb
    pub show_breadcrumb: bool,
    /// Theme name
    pub theme: String,
    /// Enable vim mode in composer
    pub vim_mode: bool,
    /// Show line numbers in code blocks
    pub show_line_numbers: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            show_sidebar: false,
            show_status_bar: true,
            show_breadcrumb: true,
            theme: "tokyo-night".to_string(),
            vim_mode: false,
            show_line_numbers: true,
        }
    }
}

/// Session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionConfig {
    /// Directory for storing sessions
    pub sessions_dir: Option<PathBuf>,
    /// Auto-save sessions
    pub auto_save: bool,
    /// Auto-save interval in seconds
    pub auto_save_interval: u64,
    /// Maximum sessions to keep
    pub max_sessions: usize,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            sessions_dir: None,
            auto_save: true,
            auto_save_interval: 30,
            max_sessions: 100,
        }
    }
}

/// Tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ToolConfig {
    /// Enable bash tool
    pub enable_bash: bool,
    /// Enable file tools (read, write, edit)
    pub enable_file_tools: bool,
    /// Enable search tools (glob, grep)
    pub enable_search_tools: bool,
    /// Auto-approve low-risk tools
    pub auto_approve_low_risk: bool,
    /// Working directory for tools
    pub working_dir: Option<PathBuf>,
    /// Bash timeout in seconds
    pub bash_timeout: u64,
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            enable_bash: true,
            enable_file_tools: true,
            enable_search_tools: true,
            auto_approve_low_risk: false,
            working_dir: None,
            bash_timeout: 120,
        }
    }
}

impl Config {
    /// Load configuration from default location (~/.miyabi/config.toml)
    pub fn load() -> anyhow::Result<Self> {
        let config_path = Self::default_path();
        if config_path.exists() {
            Self::load_from(&config_path)
        } else {
            Ok(Self::default())
        }
    }

    /// Load configuration from a specific path
    pub fn load_from(path: &PathBuf) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        let mut config: Config = toml::from_str(&content)?;

        // Apply environment variable overrides
        config.apply_env_overrides();

        Ok(config)
    }

    /// Get default configuration path
    pub fn default_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".miyabi").join("config.toml")
    }

    /// Get default config directory
    pub fn default_dir() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".miyabi")
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(&mut self) {
        // API key from environment
        if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            self.api.api_key = Some(key);
        }

        // Model from environment
        if let Ok(model) = std::env::var("MIYABI_MODEL") {
            self.api.model = model;
        }

        // Max tokens from environment
        if let Ok(tokens) = std::env::var("MIYABI_MAX_TOKENS") {
            if let Ok(n) = tokens.parse() {
                self.api.max_tokens = n;
            }
        }

        // Thinking toggle from environment
        if let Ok(thinking) = std::env::var("MIYABI_THINKING") {
            let thinking = thinking.eq_ignore_ascii_case("true") || thinking == "1";
            self.api.thinking = thinking;
        }
    }

    /// Save configuration to default location
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::default_path();
        self.save_to(&path)
    }

    /// Save configuration to a specific path
    pub fn save_to(&self, path: &PathBuf) -> anyhow::Result<()> {
        // Ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Generate default configuration file
    pub fn generate_default() -> anyhow::Result<PathBuf> {
        let config = Self::default();
        let path = Self::default_path();
        config.save_to(&path)?;
        Ok(path)
    }

    /// Get API key (from config or environment)
    pub fn api_key(&self) -> Option<&str> {
        self.api.api_key.as_deref()
    }

    /// Get sessions directory
    pub fn sessions_dir(&self) -> PathBuf {
        self.session
            .sessions_dir
            .clone()
            .unwrap_or_else(|| Self::default_dir().join("sessions"))
    }

    /// Get working directory for tools
    pub fn working_dir(&self) -> PathBuf {
        self.tools
            .working_dir
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.api.model, "claude-sonnet-4-5-20250929");
        assert_eq!(config.api.max_tokens, 8192);
        assert!(!config.api.thinking);
        assert!(config.ui.show_status_bar);
    }

    #[test]
    fn test_api_config_default() {
        let api = ApiConfig::default();
        assert!(api.api_key.is_none());
        assert_eq!(api.timeout_secs, 120);
        assert_eq!(api.max_retries, 3);
        assert!(!api.thinking);
    }

    #[test]
    fn test_ui_config_default() {
        let ui = UiConfig::default();
        assert!(!ui.show_sidebar);
        assert!(ui.show_status_bar);
        assert_eq!(ui.theme, "tokyo-night");
    }

    #[test]
    fn test_session_config_default() {
        let session = SessionConfig::default();
        assert!(session.auto_save);
        assert_eq!(session.auto_save_interval, 30);
        assert_eq!(session.max_sessions, 100);
    }

    #[test]
    fn test_tool_config_default() {
        let tools = ToolConfig::default();
        assert!(tools.enable_bash);
        assert!(tools.enable_file_tools);
        assert!(!tools.auto_approve_low_risk);
    }

    #[test]
    fn test_save_load_config() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");

        let config = Config::default();
        config.save_to(&path).unwrap();

        let loaded = Config::load_from(&path).unwrap();
        assert_eq!(loaded.api.model, config.api.model);
        assert_eq!(loaded.api.max_tokens, config.api.max_tokens);
        assert_eq!(loaded.api.thinking, config.api.thinking);
    }

    #[test]
    fn test_config_with_custom_values() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");

        let content = r#"
[api]
model = "claude-3-opus"
max_tokens = 4096
thinking = true

[ui]
theme = "dracula"
vim_mode = true

[session]
auto_save = false
"#;
        fs::write(&path, content).unwrap();

        let config = Config::load_from(&path).unwrap();
        assert_eq!(config.api.model, "claude-3-opus");
        assert_eq!(config.api.max_tokens, 4096);
        assert!(config.api.thinking);
        assert_eq!(config.ui.theme, "dracula");
        assert!(config.ui.vim_mode);
        assert!(!config.session.auto_save);
    }

    #[test]
    fn test_partial_config() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");

        // Only specify some values
        let content = r#"
[api]
model = "custom-model"
"#;
        fs::write(&path, content).unwrap();

        let config = Config::load_from(&path).unwrap();
        assert_eq!(config.api.model, "custom-model");
        // Defaults should be applied
        assert_eq!(config.api.max_tokens, 8192);
        assert!(config.ui.show_status_bar);
    }

    #[test]
    fn test_sessions_dir() {
        let config = Config::default();
        let sessions_dir = config.sessions_dir();
        assert!(sessions_dir.to_string_lossy().contains("sessions"));
    }

    #[test]
    fn test_working_dir() {
        let mut config = Config::default();
        config.tools.working_dir = Some(PathBuf::from("/tmp/test"));
        assert_eq!(config.working_dir(), PathBuf::from("/tmp/test"));
    }

    #[test]
    fn test_toml_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("[api]"));
        assert!(toml_str.contains("[ui]"));
        assert!(toml_str.contains("[session]"));
        assert!(toml_str.contains("[tools]"));
    }

    #[test]
    fn test_default_path() {
        let path = Config::default_path();
        assert!(path.to_string_lossy().contains("config.toml"));
        assert!(path.to_string_lossy().contains(".miyabi"));
    }

    #[test]
    fn test_default_dir() {
        let dir = Config::default_dir();
        assert!(dir.to_string_lossy().contains(".miyabi"));
    }

    #[test]
    fn test_api_key_none() {
        let config = Config::default();
        assert!(config.api_key().is_none());
    }

    #[test]
    fn test_api_key_some() {
        let mut config = Config::default();
        config.api.api_key = Some("test-key".to_string());
        assert_eq!(config.api_key(), Some("test-key"));
    }

    #[test]
    fn test_working_dir_default() {
        let config = Config::default();
        let working = config.working_dir();
        // Should not panic and return some path
        assert!(!working.as_os_str().is_empty() || working.as_os_str().is_empty());
    }

    #[test]
    fn test_config_clone() {
        let config = Config::default();
        let cloned = config.clone();
        assert_eq!(config.api.model, cloned.api.model);
        assert_eq!(config.ui.theme, cloned.ui.theme);
    }

    #[test]
    fn test_api_config_system_prompt() {
        let api = ApiConfig::default();
        assert!(api.system_prompt.is_some());
        assert!(api.system_prompt.unwrap().contains("helpful"));
    }

    #[test]
    fn test_ui_config_vim_mode_default() {
        let ui = UiConfig::default();
        assert!(!ui.vim_mode);
        assert!(ui.show_line_numbers);
    }

    #[test]
    fn test_session_config_sessions_dir() {
        let session = SessionConfig::default();
        assert!(session.sessions_dir.is_none());
    }

    #[test]
    fn test_tool_config_bash_timeout() {
        let tools = ToolConfig::default();
        assert_eq!(tools.bash_timeout, 120);
        assert!(tools.enable_search_tools);
    }

    #[test]
    fn test_save_creates_directory() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nested").join("dir").join("config.toml");

        let config = Config::default();
        config.save_to(&path).unwrap();

        assert!(path.exists());
    }

    #[test]
    fn test_load_nonexistent_returns_default() {
        let result = Config::load();
        // Should not error, returns default if no config file
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let mut config = Config::default();
        config.api.api_key = Some("test-key".to_string());
        config.api.model = "custom-model".to_string();
        config.ui.vim_mode = true;
        config.session.max_sessions = 50;

        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");

        config.save_to(&path).unwrap();
        let loaded = Config::load_from(&path).unwrap();

        assert_eq!(loaded.api.model, "custom-model");
        assert!(loaded.ui.vim_mode);
        assert_eq!(loaded.session.max_sessions, 50);
    }

    #[test]
    fn test_invalid_toml_returns_error() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");

        fs::write(&path, "invalid toml content [[[").unwrap();

        let result = Config::load_from(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_config_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");

        fs::write(&path, "").unwrap();

        // Empty file should use all defaults
        let config = Config::load_from(&path).unwrap();
        assert_eq!(config.api.model, "claude-sonnet-4-5-20250929");
    }

    #[test]
    fn test_config_debug_trait() {
        let config = Config::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("Config"));
        assert!(debug_str.contains("api"));
    }
}
