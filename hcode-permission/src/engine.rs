//! Permission engine.

use crate::{PermissionResult, PermissionRule};
use serde_json::Value;

/// Permission engine for checking tool permissions.
pub struct PermissionEngine {
    rules: Vec<PermissionRule>,
}

impl PermissionEngine {
    /// Create a new permission engine.
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Add a rule.
    pub fn add_rule(&mut self, rule: PermissionRule) {
        self.rules.push(rule);
    }

    /// Check permission for a tool.
    pub fn check(&self, tool: &str, _input: &Value) -> PermissionResult {
        for rule in &self.rules {
            if rule.matches(tool) {
                return rule.result();
            }
        }
        PermissionResult::allow()
    }
}

impl Default for PermissionEngine {
    fn default() -> Self {
        Self::new()
    }
}
