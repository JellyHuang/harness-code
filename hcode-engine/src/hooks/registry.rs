//! Hook registry.

use super::events::HookConfig;
use std::collections::HashMap;

/// Registry for managing hooks.
pub struct HookRegistry {
    /// Hooks by event name.
    hooks: HashMap<String, Vec<HookConfig>>,
}

impl HookRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            hooks: HashMap::new(),
        }
    }

    /// Register a hook.
    pub fn register(&mut self, config: HookConfig) {
        let entry = self.hooks.entry(config.event.clone()).or_default();
        entry.push(config);
    }

    /// Get hooks for an event.
    pub fn get_hooks(&self, event: &str) -> Vec<&HookConfig> {
        self.hooks
            .get(event)
            .map(|v| v.iter().filter(|h| h.enabled).collect())
            .unwrap_or_default()
    }

    /// Clear all hooks.
    pub fn clear(&mut self) {
        self.hooks.clear();
    }

    /// Load hooks from configuration.
    pub fn load_from_config(&mut self, configs: Vec<HookConfig>) {
        for config in configs {
            self.register(config);
        }
    }

    /// Get total hook count.
    pub fn count(&self) -> usize {
        self.hooks.values().map(|v| v.len()).sum()
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}