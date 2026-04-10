//! Plugin system for Miyabi
//!
//! Provides a flexible plugin architecture for extending Miyabi functionality.
//!
//! # Example
//!
//! ```rust
//! use mergegate_core::plugin::{Plugin, PluginManager, PluginMetadata, PluginContext, PluginResult};
//! use anyhow::Result;
//!
//! struct MyPlugin;
//!
//! impl Plugin for MyPlugin {
//!     fn metadata(&self) -> PluginMetadata {
//!         PluginMetadata {
//!             name: "my-plugin".to_string(),
//!             version: "1.0.0".to_string(),
//!             description: Some("My custom plugin".to_string()),
//!             author: Some("Miyabi Team".to_string()),
//!         }
//!     }
//!
//!     fn init(&mut self) -> Result<()> {
//!         println!("Plugin initialized!");
//!         Ok(())
//!     }
//!
//!     fn execute(&self, context: &PluginContext) -> Result<PluginResult> {
//!         Ok(PluginResult {
//!             success: true,
//!             message: Some("Plugin executed successfully".to_string()),
//!             data: None,
//!         })
//!     }
//! }
//! ```

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Plugin metadata information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginMetadata {
    /// Plugin name (unique identifier)
    pub name: String,
    /// Semantic version
    pub version: String,
    /// Optional description
    pub description: Option<String>,
    /// Optional author
    pub author: Option<String>,
}

/// Plugin execution context
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginContext {
    /// Plugin parameters
    #[serde(default)]
    pub params: HashMap<String, serde_json::Value>,
    /// Working directory
    pub working_dir: Option<String>,
    /// Environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// Plugin execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResult {
    /// Execution success status
    pub success: bool,
    /// Optional result message
    pub message: Option<String>,
    /// Optional result data
    pub data: Option<serde_json::Value>,
}

/// Core plugin trait
///
/// All plugins must implement this trait to be registered with the PluginManager.
pub trait Plugin: Send + Sync {
    /// Returns plugin metadata
    fn metadata(&self) -> PluginMetadata;

    /// Initializes the plugin
    ///
    /// Called once when the plugin is registered.
    fn init(&mut self) -> Result<()>;

    /// Executes the plugin with the given context
    fn execute(&self, context: &PluginContext) -> Result<PluginResult>;

    /// Cleans up plugin resources
    ///
    /// Called when the plugin is unregistered or the manager is dropped.
    fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Plugin lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginState {
    /// Plugin is registered but not initialized
    Registered,
    /// Plugin is initialized and ready
    Initialized,
    /// Plugin encountered an error
    Error,
    /// Plugin is shut down
    Shutdown,
}

/// Internal plugin wrapper
struct PluginEntry {
    plugin: Box<dyn Plugin>,
    state: PluginState,
}

/// Plugin manager
///
/// Manages plugin registration, initialization, and execution.
#[derive(Clone)]
pub struct PluginManager {
    plugins: Arc<RwLock<HashMap<String, PluginEntry>>>,
}

impl PluginManager {
    /// Creates a new plugin manager
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Registers a plugin
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - A plugin with the same name already exists
    /// - Plugin initialization fails
    pub fn register(&self, mut plugin: Box<dyn Plugin>) -> Result<()> {
        let metadata = plugin.metadata();
        let name = metadata.name.clone();

        // Check if plugin already exists
        {
            let plugins = self.plugins.read().unwrap();
            if plugins.contains_key(&name) {
                return Err(anyhow!("Plugin '{}' is already registered", name));
            }
        }

        // Initialize plugin
        plugin.init()?;

        // Store plugin
        let mut plugins = self.plugins.write().unwrap();
        plugins.insert(
            name.clone(),
            PluginEntry {
                plugin,
                state: PluginState::Initialized,
            },
        );

        Ok(())
    }

    /// Unregisters a plugin
    ///
    /// # Errors
    ///
    /// Returns an error if the plugin doesn't exist
    pub fn unregister(&self, name: &str) -> Result<()> {
        let mut plugins = self.plugins.write().unwrap();

        let mut entry = plugins
            .remove(name)
            .ok_or_else(|| anyhow!("Plugin '{}' not found", name))?;

        entry.plugin.shutdown()?;
        entry.state = PluginState::Shutdown;

        Ok(())
    }

    /// Executes a plugin with the given context
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The plugin doesn't exist
    /// - The plugin is not initialized
    /// - Plugin execution fails
    pub fn execute(&self, name: &str, context: &PluginContext) -> Result<PluginResult> {
        let plugins = self.plugins.read().unwrap();

        let entry = plugins
            .get(name)
            .ok_or_else(|| anyhow!("Plugin '{}' not found", name))?;

        if entry.state != PluginState::Initialized {
            return Err(anyhow!(
                "Plugin '{}' is not initialized (state: {:?})",
                name,
                entry.state
            ));
        }

        entry.plugin.execute(context)
    }

    /// Lists all registered plugins
    pub fn list_plugins(&self) -> Vec<PluginMetadata> {
        let plugins = self.plugins.read().unwrap();
        plugins
            .values()
            .map(|entry| entry.plugin.metadata())
            .collect()
    }

    /// Gets plugin metadata
    ///
    /// # Errors
    ///
    /// Returns an error if the plugin doesn't exist
    pub fn get_metadata(&self, name: &str) -> Result<PluginMetadata> {
        let plugins = self.plugins.read().unwrap();
        plugins
            .get(name)
            .map(|entry| entry.plugin.metadata())
            .ok_or_else(|| anyhow!("Plugin '{}' not found", name))
    }

    /// Gets plugin state
    ///
    /// # Errors
    ///
    /// Returns an error if the plugin doesn't exist
    pub fn get_state(&self, name: &str) -> Result<PluginState> {
        let plugins = self.plugins.read().unwrap();
        plugins
            .get(name)
            .map(|entry| entry.state)
            .ok_or_else(|| anyhow!("Plugin '{}' not found", name))
    }

    /// Checks if a plugin exists
    pub fn has_plugin(&self, name: &str) -> bool {
        let plugins = self.plugins.read().unwrap();
        plugins.contains_key(name)
    }

    /// Gets the number of registered plugins
    pub fn count(&self) -> usize {
        let plugins = self.plugins.read().unwrap();
        plugins.len()
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for PluginManager {
    fn drop(&mut self) {
        // Shutdown all plugins
        if let Ok(mut plugins) = self.plugins.write() {
            for (name, entry) in plugins.iter_mut() {
                if let Err(e) = entry.plugin.shutdown() {
                    eprintln!("Error shutting down plugin '{}': {}", name, e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPlugin {
        name: String,
        initialized: bool,
    }

    impl TestPlugin {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                initialized: false,
            }
        }
    }

    impl Plugin for TestPlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata {
                name: self.name.clone(),
                version: "1.0.0".to_string(),
                description: Some("Test plugin".to_string()),
                author: Some("Test Author".to_string()),
            }
        }

        fn init(&mut self) -> Result<()> {
            self.initialized = true;
            Ok(())
        }

        fn execute(&self, _context: &PluginContext) -> Result<PluginResult> {
            Ok(PluginResult {
                success: true,
                message: Some(format!("Plugin '{}' executed", self.name)),
                data: None,
            })
        }
    }

    #[test]
    fn test_register_plugin() {
        let manager = PluginManager::new();
        let plugin = Box::new(TestPlugin::new("test-plugin"));

        assert!(manager.register(plugin).is_ok());
        assert_eq!(manager.count(), 1);
        assert!(manager.has_plugin("test-plugin"));
    }

    #[test]
    fn test_duplicate_registration() {
        let manager = PluginManager::new();
        let plugin1 = Box::new(TestPlugin::new("test-plugin"));
        let plugin2 = Box::new(TestPlugin::new("test-plugin"));

        assert!(manager.register(plugin1).is_ok());
        assert!(manager.register(plugin2).is_err());
    }

    #[test]
    fn test_execute_plugin() {
        let manager = PluginManager::new();
        let plugin = Box::new(TestPlugin::new("test-plugin"));

        manager.register(plugin).unwrap();

        let context = PluginContext::default();
        let result = manager.execute("test-plugin", &context).unwrap();

        assert!(result.success);
        assert!(result.message.is_some());
    }

    #[test]
    fn test_unregister_plugin() {
        let manager = PluginManager::new();
        let plugin = Box::new(TestPlugin::new("test-plugin"));

        manager.register(plugin).unwrap();
        assert_eq!(manager.count(), 1);

        manager.unregister("test-plugin").unwrap();
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_list_plugins() {
        let manager = PluginManager::new();
        let plugin1 = Box::new(TestPlugin::new("plugin-1"));
        let plugin2 = Box::new(TestPlugin::new("plugin-2"));

        manager.register(plugin1).unwrap();
        manager.register(plugin2).unwrap();

        let plugins = manager.list_plugins();
        assert_eq!(plugins.len(), 2);
    }

    #[test]
    fn test_get_metadata() {
        let manager = PluginManager::new();
        let plugin = Box::new(TestPlugin::new("test-plugin"));

        manager.register(plugin).unwrap();

        let metadata = manager.get_metadata("test-plugin").unwrap();
        assert_eq!(metadata.name, "test-plugin");
        assert_eq!(metadata.version, "1.0.0");
    }

    #[test]
    fn test_get_state() {
        let manager = PluginManager::new();
        let plugin = Box::new(TestPlugin::new("test-plugin"));

        manager.register(plugin).unwrap();

        let state = manager.get_state("test-plugin").unwrap();
        assert_eq!(state, PluginState::Initialized);
    }

    #[test]
    fn test_plugin_not_found() {
        let manager = PluginManager::new();
        let context = PluginContext::default();

        assert!(manager.execute("nonexistent", &context).is_err());
        assert!(manager.get_metadata("nonexistent").is_err());
        assert!(manager.get_state("nonexistent").is_err());
        assert!(manager.unregister("nonexistent").is_err());
    }
}
