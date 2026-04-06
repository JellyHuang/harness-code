//! Permission rules.

use hcode_types::PermissionAction;

/// A permission rule.
#[derive(Debug, Clone)]
pub struct PermissionRule {
    pattern: String,
    action: PermissionAction,
    reason: Option<String>,
}

impl PermissionRule {
    /// Create a new rule.
    pub fn new(pattern: impl Into<String>, action: PermissionAction) -> Self {
        Self {
            pattern: pattern.into(),
            action,
            reason: None,
        }
    }

    /// Add a reason.
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Check if this rule matches a tool name.
    pub fn matches(&self, tool: &str) -> bool {
        self.pattern == "*" || self.pattern == tool
    }

    /// Get the result for this rule.
    pub fn result(&self) -> crate::PermissionResult {
        match self.action {
            PermissionAction::Allow => crate::PermissionResult::allow(),
            PermissionAction::Deny => crate::PermissionResult::deny(
                self.reason
                    .clone()
                    .unwrap_or_else(|| "Denied by rule".to_string()),
            ),
            PermissionAction::Ask => crate::PermissionResult::ask(
                self.reason
                    .clone()
                    .unwrap_or_else(|| "Confirmation required".to_string()),
            ),
        }
    }
}
