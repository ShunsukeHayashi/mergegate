//! Project-specific rules support (.miyabirules)
//!
//! This module provides support for loading and applying custom rules from `.miyabirules` files.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Error types for rules operations
#[derive(Error, Debug)]
pub enum RulesError {
    /// File not found
    #[error("Rules file not found: {0}")]
    FileNotFound(PathBuf),

    /// Parse error
    #[error("Failed to parse rules file: {0}")]
    ParseError(String),

    /// Validation error
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, RulesError>;

/// A single rule with pattern matching and suggestion
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Rule {
    /// Rule name
    pub name: String,

    /// Pattern to match (regex)
    #[serde(default)]
    pub pattern: Option<String>,

    /// Suggestion message
    pub suggestion: String,

    /// File extension filters (e.g., ["rs", "toml"])
    #[serde(default)]
    pub file_extensions: Vec<String>,

    /// Severity: "info", "warning", "error"
    #[serde(default = "default_severity")]
    pub severity: String,

    /// Whether this rule is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_severity() -> String {
    "info".to_string()
}

fn default_enabled() -> bool {
    true
}

impl Rule {
    /// Check if this rule applies to a given file extension
    pub fn applies_to_file(&self, file_path: &Path) -> bool {
        if self.file_extensions.is_empty() {
            return true;
        }

        if let Some(ext) = file_path.extension() {
            let ext_str = ext.to_string_lossy().to_string();
            self.file_extensions.iter().any(|e| e == &ext_str)
        } else {
            false
        }
    }

    /// Check if this rule matches a given line of code
    pub fn matches(&self, line: &str) -> bool {
        if let Some(pattern) = &self.pattern {
            line.contains(pattern)
        } else {
            false
        }
    }
}

/// Agent-specific preferences
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AgentPreferences {
    /// Code style preference
    #[serde(default)]
    pub style: Option<String>,

    /// Error handling strategy
    #[serde(default)]
    pub error_handling: Option<String>,

    /// Minimum quality score
    #[serde(default)]
    pub min_score: Option<u8>,

    /// Enable strict Clippy checks
    #[serde(default)]
    pub clippy_strict: Option<bool>,

    /// Custom agent-specific settings
    #[serde(flatten)]
    pub custom: HashMap<String, serde_json::Value>,
}

/// Root configuration structure for .miyabirules
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MiyabiRules {
    /// Version of the rules format
    #[serde(default = "default_version")]
    pub version: u32,

    /// List of rules
    #[serde(default)]
    pub rules: Vec<Rule>,

    /// Agent preferences by agent type
    #[serde(default)]
    pub agent_preferences: HashMap<String, AgentPreferences>,

    /// Global settings
    #[serde(default)]
    pub settings: HashMap<String, serde_json::Value>,
}

fn default_version() -> u32 {
    1
}

impl Default for MiyabiRules {
    fn default() -> Self {
        Self {
            version: 1,
            rules: Vec::new(),
            agent_preferences: HashMap::new(),
            settings: HashMap::new(),
        }
    }
}

impl MiyabiRules {
    /// Create a new empty rules configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate the rules configuration
    pub fn validate(&self) -> Result<()> {
        if self.version != 1 {
            return Err(RulesError::ValidationError(format!(
                "Unsupported version: {}. Only version 1 is supported.",
                self.version
            )));
        }

        for rule in &self.rules {
            if rule.name.is_empty() {
                return Err(RulesError::ValidationError(
                    "Rule name cannot be empty".to_string(),
                ));
            }

            if rule.suggestion.is_empty() {
                return Err(RulesError::ValidationError(format!(
                    "Rule '{}' must have a suggestion",
                    rule.name
                )));
            }

            match rule.severity.as_str() {
                "info" | "warning" | "error" => {}
                _ => {
                    return Err(RulesError::ValidationError(format!(
                        "Invalid severity '{}' for rule '{}'. Must be: info, warning, or error",
                        rule.severity, rule.name
                    )))
                }
            }
        }

        Ok(())
    }

    /// Get rules that apply to a specific file
    pub fn rules_for_file(&self, file_path: &Path) -> Vec<&Rule> {
        self.rules
            .iter()
            .filter(|r| r.enabled && r.applies_to_file(file_path))
            .collect()
    }

    /// Get agent preferences for a specific agent type
    pub fn get_agent_preferences(&self, agent_type: &str) -> Option<&AgentPreferences> {
        self.agent_preferences.get(agent_type)
    }

    /// Get a global setting value
    pub fn get_setting(&self, key: &str) -> Option<&serde_json::Value> {
        self.settings.get(key)
    }
}

/// Rules loader for loading .miyabirules files
pub struct RulesLoader {
    /// Root directory to search for .miyabirules
    root_dir: PathBuf,
}

impl RulesLoader {
    /// Create a new RulesLoader
    pub fn new(root_dir: PathBuf) -> Self {
        Self { root_dir }
    }

    /// Find .miyabirules file in the directory hierarchy
    pub fn find_rules_file(&self) -> Option<PathBuf> {
        let mut current = self.root_dir.clone();

        loop {
            let rules_path = current.join(".miyabirules");
            if rules_path.exists() {
                return Some(rules_path);
            }

            let rules_yaml = current.join(".miyabirules.yaml");
            if rules_yaml.exists() {
                return Some(rules_yaml);
            }

            let rules_yml = current.join(".miyabirules.yml");
            if rules_yml.exists() {
                return Some(rules_yml);
            }

            if !current.pop() {
                break;
            }
        }

        None
    }

    /// Load rules from .miyabirules file
    pub fn load(&self) -> Result<Option<MiyabiRules>> {
        let rules_path = match self.find_rules_file() {
            Some(path) => path,
            None => return Ok(None),
        };

        let content = fs::read_to_string(&rules_path)?;

        let rules: MiyabiRules = serde_yaml::from_str(&content).map_err(|e| {
            RulesError::ParseError(format!("Failed to parse {}: {}", rules_path.display(), e))
        })?;

        rules.validate()?;

        Ok(Some(rules))
    }

    /// Load rules or return default if not found
    pub fn load_or_default(&self) -> Result<MiyabiRules> {
        self.load().map(|opt| opt.unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_rule_applies_to_file() {
        let rule = Rule {
            name: "Rust rule".to_string(),
            pattern: None,
            suggestion: "Test".to_string(),
            file_extensions: vec!["rs".to_string()],
            severity: "info".to_string(),
            enabled: true,
        };

        assert!(rule.applies_to_file(Path::new("main.rs")));
        assert!(!rule.applies_to_file(Path::new("main.py")));
    }

    #[test]
    fn test_rule_matches() {
        let rule = Rule {
            name: "Test".to_string(),
            pattern: Some("async".to_string()),
            suggestion: "Test".to_string(),
            file_extensions: vec![],
            severity: "info".to_string(),
            enabled: true,
        };

        assert!(rule.matches("async fn test() {}"));
        assert!(!rule.matches("fn test() {}"));
    }

    #[test]
    fn test_rules_validation() {
        let rules = MiyabiRules::default();
        assert!(rules.validate().is_ok());
    }

    #[test]
    fn test_rules_loader_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let loader = RulesLoader::new(temp_dir.path().to_path_buf());
        let rules = loader.load().unwrap();
        assert!(rules.is_none());
    }

    #[test]
    fn test_rule_default_values() {
        let yaml = r#"
name: test-rule
suggestion: Do something
"#;
        let rule: Rule = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(rule.name, "test-rule");
        assert_eq!(rule.severity, "info");
        assert!(rule.enabled);
        assert!(rule.file_extensions.is_empty());
        assert!(rule.pattern.is_none());
    }

    #[test]
    fn test_rule_applies_to_file_no_extensions() {
        let rule = Rule {
            name: "Any file".to_string(),
            pattern: None,
            suggestion: "Test".to_string(),
            file_extensions: vec![],
            severity: "info".to_string(),
            enabled: true,
        };

        assert!(rule.applies_to_file(Path::new("main.rs")));
        assert!(rule.applies_to_file(Path::new("main.py")));
        assert!(rule.applies_to_file(Path::new("file.txt")));
    }

    #[test]
    fn test_rule_applies_to_file_no_extension() {
        let rule = Rule {
            name: "Rust rule".to_string(),
            pattern: None,
            suggestion: "Test".to_string(),
            file_extensions: vec!["rs".to_string()],
            severity: "info".to_string(),
            enabled: true,
        };

        assert!(!rule.applies_to_file(Path::new("Makefile")));
    }

    #[test]
    fn test_rule_matches_no_pattern() {
        let rule = Rule {
            name: "Test".to_string(),
            pattern: None,
            suggestion: "Test".to_string(),
            file_extensions: vec![],
            severity: "info".to_string(),
            enabled: true,
        };

        assert!(!rule.matches("any content"));
    }

    #[test]
    fn test_agent_preferences_default() {
        let prefs = AgentPreferences::default();
        assert!(prefs.style.is_none());
        assert!(prefs.error_handling.is_none());
        assert!(prefs.min_score.is_none());
        assert!(prefs.clippy_strict.is_none());
        assert!(prefs.custom.is_empty());
    }

    #[test]
    fn test_agent_preferences_full() {
        let yaml = r#"
style: functional
error_handling: propagate
min_score: 80
clippy_strict: true
custom_key: "custom_value"
"#;
        let prefs: AgentPreferences = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(prefs.style, Some("functional".to_string()));
        assert_eq!(prefs.error_handling, Some("propagate".to_string()));
        assert_eq!(prefs.min_score, Some(80));
        assert_eq!(prefs.clippy_strict, Some(true));
        assert!(prefs.custom.contains_key("custom_key"));
    }

    #[test]
    fn test_miyabi_rules_new() {
        let rules = MiyabiRules::new();
        assert_eq!(rules.version, 1);
        assert!(rules.rules.is_empty());
        assert!(rules.agent_preferences.is_empty());
        assert!(rules.settings.is_empty());
    }

    #[test]
    fn test_miyabi_rules_validate_invalid_version() {
        let rules = MiyabiRules {
            version: 2,
            ..Default::default()
        };
        assert!(rules.validate().is_err());
    }

    #[test]
    fn test_miyabi_rules_validate_empty_name() {
        let rules = MiyabiRules {
            rules: vec![Rule {
                name: "".to_string(),
                pattern: None,
                suggestion: "Test".to_string(),
                file_extensions: vec![],
                severity: "info".to_string(),
                enabled: true,
            }],
            ..Default::default()
        };
        assert!(rules.validate().is_err());
    }

    #[test]
    fn test_miyabi_rules_validate_empty_suggestion() {
        let rules = MiyabiRules {
            rules: vec![Rule {
                name: "Test".to_string(),
                pattern: None,
                suggestion: "".to_string(),
                file_extensions: vec![],
                severity: "info".to_string(),
                enabled: true,
            }],
            ..Default::default()
        };
        assert!(rules.validate().is_err());
    }

    #[test]
    fn test_miyabi_rules_validate_invalid_severity() {
        let rules = MiyabiRules {
            rules: vec![Rule {
                name: "Test".to_string(),
                pattern: None,
                suggestion: "Do something".to_string(),
                file_extensions: vec![],
                severity: "critical".to_string(),
                enabled: true,
            }],
            ..Default::default()
        };
        assert!(rules.validate().is_err());
    }

    #[test]
    fn test_miyabi_rules_validate_valid_severities() {
        for severity in &["info", "warning", "error"] {
            let rules = MiyabiRules {
                rules: vec![Rule {
                    name: "Test".to_string(),
                    pattern: None,
                    suggestion: "Do something".to_string(),
                    file_extensions: vec![],
                    severity: severity.to_string(),
                    enabled: true,
                }],
                ..Default::default()
            };
            assert!(rules.validate().is_ok());
        }
    }

    #[test]
    fn test_miyabi_rules_rules_for_file() {
        let rules = MiyabiRules {
            rules: vec![
                Rule {
                    name: "Rust rule".to_string(),
                    pattern: Some("unwrap".to_string()),
                    suggestion: "Use expect".to_string(),
                    file_extensions: vec!["rs".to_string()],
                    severity: "warning".to_string(),
                    enabled: true,
                },
                Rule {
                    name: "Python rule".to_string(),
                    pattern: Some("import".to_string()),
                    suggestion: "Check imports".to_string(),
                    file_extensions: vec!["py".to_string()],
                    severity: "info".to_string(),
                    enabled: true,
                },
                Rule {
                    name: "Disabled rule".to_string(),
                    pattern: Some("test".to_string()),
                    suggestion: "Disabled".to_string(),
                    file_extensions: vec!["rs".to_string()],
                    severity: "info".to_string(),
                    enabled: false,
                },
            ],
            ..Default::default()
        };

        let rust_rules = rules.rules_for_file(Path::new("main.rs"));
        assert_eq!(rust_rules.len(), 1);
        assert_eq!(rust_rules[0].name, "Rust rule");
    }

    #[test]
    fn test_miyabi_rules_get_agent_preferences() {
        let mut prefs = HashMap::new();
        prefs.insert(
            "coder".to_string(),
            AgentPreferences {
                style: Some("functional".to_string()),
                ..Default::default()
            },
        );

        let rules = MiyabiRules {
            agent_preferences: prefs,
            ..Default::default()
        };

        let coder_prefs = rules.get_agent_preferences("coder");
        assert!(coder_prefs.is_some());
        assert_eq!(coder_prefs.unwrap().style, Some("functional".to_string()));

        let other_prefs = rules.get_agent_preferences("other");
        assert!(other_prefs.is_none());
    }

    #[test]
    fn test_miyabi_rules_get_setting() {
        let mut settings = HashMap::new();
        settings.insert("key".to_string(), serde_json::json!("value"));

        let rules = MiyabiRules {
            settings,
            ..Default::default()
        };

        let value = rules.get_setting("key");
        assert!(value.is_some());
        assert_eq!(value.unwrap(), &serde_json::json!("value"));

        let missing = rules.get_setting("missing");
        assert!(missing.is_none());
    }

    #[test]
    fn test_rules_loader_find_rules_file() {
        let temp_dir = TempDir::new().unwrap();
        let loader = RulesLoader::new(temp_dir.path().to_path_buf());

        // No file exists
        assert!(loader.find_rules_file().is_none());

        // Create .miyabirules
        std::fs::write(temp_dir.path().join(".miyabirules"), "version: 1").unwrap();
        let found = loader.find_rules_file();
        assert!(found.is_some());
        assert!(found.unwrap().ends_with(".miyabirules"));
    }

    #[test]
    fn test_rules_loader_find_yaml_extension() {
        let temp_dir = TempDir::new().unwrap();
        let loader = RulesLoader::new(temp_dir.path().to_path_buf());

        // Create .miyabirules.yaml
        std::fs::write(temp_dir.path().join(".miyabirules.yaml"), "version: 1").unwrap();
        let found = loader.find_rules_file();
        assert!(found.is_some());
        assert!(found.unwrap().ends_with(".miyabirules.yaml"));
    }

    #[test]
    fn test_rules_loader_find_yml_extension() {
        let temp_dir = TempDir::new().unwrap();
        let loader = RulesLoader::new(temp_dir.path().to_path_buf());

        // Create .miyabirules.yml
        std::fs::write(temp_dir.path().join(".miyabirules.yml"), "version: 1").unwrap();
        let found = loader.find_rules_file();
        assert!(found.is_some());
        assert!(found.unwrap().ends_with(".miyabirules.yml"));
    }

    #[test]
    fn test_rules_loader_load() {
        let temp_dir = TempDir::new().unwrap();
        let loader = RulesLoader::new(temp_dir.path().to_path_buf());

        let yaml = r#"
version: 1
rules:
  - name: test-rule
    suggestion: Do something
    severity: warning
settings:
  key: value
"#;
        std::fs::write(temp_dir.path().join(".miyabirules"), yaml).unwrap();

        let rules = loader.load().unwrap().unwrap();
        assert_eq!(rules.version, 1);
        assert_eq!(rules.rules.len(), 1);
        assert_eq!(rules.rules[0].name, "test-rule");
    }

    #[test]
    fn test_rules_loader_load_or_default() {
        let temp_dir = TempDir::new().unwrap();
        let loader = RulesLoader::new(temp_dir.path().to_path_buf());

        // No file - returns default
        let rules = loader.load_or_default().unwrap();
        assert_eq!(rules.version, 1);
        assert!(rules.rules.is_empty());
    }

    #[test]
    fn test_rules_loader_parse_error() {
        let temp_dir = TempDir::new().unwrap();
        let loader = RulesLoader::new(temp_dir.path().to_path_buf());

        // Invalid YAML
        std::fs::write(
            temp_dir.path().join(".miyabirules"),
            "invalid: yaml: content:",
        )
        .unwrap();

        let result = loader.load();
        assert!(result.is_err());
    }

    #[test]
    fn test_rules_error_display() {
        let err = RulesError::FileNotFound(PathBuf::from("/test/path"));
        assert!(err.to_string().contains("/test/path"));

        let err = RulesError::ParseError("syntax error".to_string());
        assert!(err.to_string().contains("syntax error"));

        let err = RulesError::ValidationError("invalid".to_string());
        assert!(err.to_string().contains("invalid"));
    }

    #[test]
    fn test_miyabi_rules_serialization() {
        let rules = MiyabiRules {
            rules: vec![Rule {
                name: "Test".to_string(),
                pattern: Some("pattern".to_string()),
                suggestion: "Suggestion".to_string(),
                file_extensions: vec!["rs".to_string()],
                severity: "warning".to_string(),
                enabled: true,
            }],
            ..Default::default()
        };

        let yaml = serde_yaml::to_string(&rules).unwrap();
        assert!(yaml.contains("Test"));
        assert!(yaml.contains("pattern"));
        assert!(yaml.contains("warning"));

        let deserialized: MiyabiRules = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(deserialized, rules);
    }
}
