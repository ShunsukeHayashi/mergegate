//! Feature Flag Management System
//!
//! Provides a flexible feature flag system for gradual rollout and A/B testing.
//!
//! # Examples
//!
//! ```rust
//! use miyabi_core::feature_flags::FeatureFlagManager;
//!
//! let manager = FeatureFlagManager::new();
//! manager.set_flag("new_architecture", true);
//!
//! if manager.is_enabled("new_architecture") {
//!     // Use new architecture
//! } else {
//!     // Use old architecture
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Feature flag configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlag {
    /// Flag name
    pub name: String,
    /// Enabled status
    pub enabled: bool,
    /// Optional description
    pub description: Option<String>,
    /// Optional rollout percentage (0.0 to 1.0)
    pub rollout_percentage: Option<f64>,
}

/// Feature flag manager for controlling feature rollout
#[derive(Debug, Clone)]
pub struct FeatureFlagManager {
    /// Internal flag storage (thread-safe)
    flags: Arc<RwLock<HashMap<String, FeatureFlag>>>,
}

impl Default for FeatureFlagManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FeatureFlagManager {
    /// Create a new feature flag manager
    pub fn new() -> Self {
        Self {
            flags: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if a feature flag is enabled
    ///
    /// # Arguments
    ///
    /// * `flag_name` - Name of the feature flag
    ///
    /// # Returns
    ///
    /// `true` if the flag exists and is enabled, `false` otherwise
    pub fn is_enabled(&self, flag_name: &str) -> bool {
        let flags = self.flags.read().unwrap();
        flags.get(flag_name).map(|flag| flag.enabled).unwrap_or(false)
    }

    /// Set a feature flag
    ///
    /// # Arguments
    ///
    /// * `flag_name` - Name of the feature flag
    /// * `enabled` - Whether the flag should be enabled
    pub fn set_flag(&self, flag_name: impl Into<String>, enabled: bool) {
        let mut flags = self.flags.write().unwrap();
        let name = flag_name.into();
        flags.insert(
            name.clone(),
            FeatureFlag {
                name,
                enabled,
                description: None,
                rollout_percentage: None,
            },
        );
    }

    /// Set a feature flag with description and rollout percentage
    ///
    /// # Arguments
    ///
    /// * `flag_name` - Name of the feature flag
    /// * `enabled` - Whether the flag should be enabled
    /// * `description` - Optional description of the flag
    /// * `rollout_percentage` - Optional rollout percentage (0.0 to 1.0)
    pub fn set_flag_with_options(
        &self,
        flag_name: impl Into<String>,
        enabled: bool,
        description: Option<String>,
        rollout_percentage: Option<f64>,
    ) {
        let mut flags = self.flags.write().unwrap();
        let name = flag_name.into();
        flags.insert(
            name.clone(),
            FeatureFlag {
                name,
                enabled,
                description,
                rollout_percentage,
            },
        );
    }

    /// Remove a feature flag
    ///
    /// # Arguments
    ///
    /// * `flag_name` - Name of the feature flag to remove
    pub fn remove_flag(&self, flag_name: &str) {
        let mut flags = self.flags.write().unwrap();
        flags.remove(flag_name);
    }

    /// Get all feature flags
    ///
    /// # Returns
    ///
    /// A vector of all feature flags
    pub fn get_all_flags(&self) -> Vec<FeatureFlag> {
        let flags = self.flags.read().unwrap();
        flags.values().cloned().collect()
    }

    /// Get a specific feature flag
    ///
    /// # Arguments
    ///
    /// * `flag_name` - Name of the feature flag
    ///
    /// # Returns
    ///
    /// The feature flag if it exists, None otherwise
    pub fn get_flag(&self, flag_name: &str) -> Option<FeatureFlag> {
        let flags = self.flags.read().unwrap();
        flags.get(flag_name).cloned()
    }

    /// Load feature flags from a HashMap
    ///
    /// # Arguments
    ///
    /// * `config` - HashMap of flag names to enabled status
    pub fn load_from_map(&self, config: HashMap<String, bool>) {
        let mut flags = self.flags.write().unwrap();
        for (name, enabled) in config {
            flags.insert(
                name.clone(),
                FeatureFlag {
                    name,
                    enabled,
                    description: None,
                    rollout_percentage: None,
                },
            );
        }
    }

    /// Load feature flags from detailed configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Vector of FeatureFlag configurations
    pub fn load_from_config(&self, config: Vec<FeatureFlag>) {
        let mut flags = self.flags.write().unwrap();
        for flag in config {
            flags.insert(flag.name.clone(), flag);
        }
    }

    /// Export all flags to a HashMap
    ///
    /// # Returns
    ///
    /// HashMap of flag names to enabled status
    pub fn export_to_map(&self) -> HashMap<String, bool> {
        let flags = self.flags.read().unwrap();
        flags.iter().map(|(name, flag)| (name.clone(), flag.enabled)).collect()
    }

    /// Clear all feature flags
    pub fn clear(&self) {
        let mut flags = self.flags.write().unwrap();
        flags.clear();
    }

    /// Get the number of feature flags
    ///
    /// # Returns
    ///
    /// The number of flags currently registered
    pub fn count(&self) -> usize {
        let flags = self.flags.read().unwrap();
        flags.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_flag() {
        let manager = FeatureFlagManager::new();

        // Initially disabled
        assert!(!manager.is_enabled("new_feature"));

        // Enable flag
        manager.set_flag("new_feature", true);
        assert!(manager.is_enabled("new_feature"));

        // Disable flag
        manager.set_flag("new_feature", false);
        assert!(!manager.is_enabled("new_feature"));
    }

    #[test]
    fn test_flag_with_options() {
        let manager = FeatureFlagManager::new();

        manager.set_flag_with_options(
            "beta_feature",
            true,
            Some("Beta testing feature".to_string()),
            Some(0.5),
        );

        assert!(manager.is_enabled("beta_feature"));

        let flag = manager.get_flag("beta_feature").unwrap();
        assert_eq!(flag.description, Some("Beta testing feature".to_string()));
        assert_eq!(flag.rollout_percentage, Some(0.5));
    }

    #[test]
    fn test_load_from_map() {
        let manager = FeatureFlagManager::new();

        let mut config = HashMap::new();
        config.insert("flag1".to_string(), true);
        config.insert("flag2".to_string(), false);

        manager.load_from_map(config);

        assert!(manager.is_enabled("flag1"));
        assert!(!manager.is_enabled("flag2"));
        assert_eq!(manager.count(), 2);
    }

    #[test]
    fn test_get_all_flags() {
        let manager = FeatureFlagManager::new();

        manager.set_flag("flag1", true);
        manager.set_flag("flag2", false);

        let all_flags = manager.get_all_flags();
        assert_eq!(all_flags.len(), 2);
    }

    #[test]
    fn test_remove_flag() {
        let manager = FeatureFlagManager::new();

        manager.set_flag("temp_flag", true);
        assert!(manager.is_enabled("temp_flag"));

        manager.remove_flag("temp_flag");
        assert!(!manager.is_enabled("temp_flag"));
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_export_to_map() {
        let manager = FeatureFlagManager::new();

        manager.set_flag("flag1", true);
        manager.set_flag("flag2", false);

        let exported = manager.export_to_map();
        assert_eq!(exported.get("flag1"), Some(&true));
        assert_eq!(exported.get("flag2"), Some(&false));
    }

    #[test]
    fn test_clear() {
        let manager = FeatureFlagManager::new();

        manager.set_flag("flag1", true);
        manager.set_flag("flag2", true);
        assert_eq!(manager.count(), 2);

        manager.clear();
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_thread_safety() {
        use std::thread;

        let manager = FeatureFlagManager::new();
        let manager_clone = manager.clone();

        // Spawn thread to set flag
        let handle = thread::spawn(move || {
            manager_clone.set_flag("concurrent_flag", true);
        });

        handle.join().unwrap();

        // Check flag is set
        assert!(manager.is_enabled("concurrent_flag"));
    }
}
