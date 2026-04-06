//! Permission types for tool access control.

use serde::{Deserialize, Serialize};

/// Result of a permission check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionResult {
    /// Permission granted.
    Allow,
    /// Permission denied with a reason.
    Deny { reason: String },
    /// User confirmation required.
    Ask { message: String },
}

impl PermissionResult {
    /// Create an allow result.
    pub fn allow() -> Self {
        Self::Allow
    }

    /// Create a deny result.
    pub fn deny(reason: impl Into<String>) -> Self {
        Self::Deny {
            reason: reason.into(),
        }
    }

    /// Create an ask result.
    pub fn ask(message: impl Into<String>) -> Self {
        Self::Ask {
            message: message.into(),
        }
    }

    /// Check if permission is allowed.
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allow)
    }

    /// Check if permission is denied.
    pub fn is_denied(&self) -> bool {
        matches!(self, Self::Deny { .. })
    }

    /// Check if user confirmation is needed.
    pub fn needs_confirmation(&self) -> bool {
        matches!(self, Self::Ask { .. })
    }
}

/// Permission mode for an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionMode {
    /// Ask for permission on every operation.
    Ask,
    /// Allow all operations without asking.
    Allow,
    /// Deny all operations by default.
    Deny,
}

/// Permission rule for a specific resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRule {
    /// Pattern to match (e.g., file path, command name).
    pub pattern: String,
    /// Action to take when pattern matches.
    pub action: PermissionAction,
    /// Optional reason for the rule.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Action to take for a permission rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionAction {
    /// Allow the operation.
    Allow,
    /// Deny the operation.
    Deny,
    /// Ask for confirmation.
    Ask,
}
