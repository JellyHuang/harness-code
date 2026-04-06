//! Tool trait definition.

use async_trait::async_trait;
use hcode_permission::PermissionResult;
use hcode_types::ToolResult;
use serde_json::Value;
use std::path::PathBuf;

/// Tool trait for implementing tools.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get the tool name.
    fn name(&self) -> &str;

    /// Get the tool description.
    fn description(&self) -> &str {
        "No description available"
    }

    /// Get the input JSON schema.
    fn input_schema(&self) -> &Value;

    /// Whether the tool is read-only.
    fn is_read_only(&self) -> bool {
        false
    }

    /// Whether the tool is safe to run in parallel.
    fn is_concurrency_safe(&self) -> bool {
        false
    }

    /// Check permissions for this tool.
    async fn check_permissions(
        &self,
        _input: &Value,
        _context: &ToolContext,
    ) -> Result<PermissionResult, ToolError> {
        Ok(PermissionResult::allow())
    }

    /// Execute the tool.
    async fn call(&self, input: Value, context: ToolContext) -> Result<ToolResult, ToolError>;
}

/// Context for tool execution.
#[derive(Debug, Clone)]
pub struct ToolContext {
    pub working_dir: PathBuf,
    pub session_id: String,
    pub tool_use_id: String,
}

impl ToolContext {
    pub fn new(
        working_dir: PathBuf,
        session_id: impl Into<String>,
        tool_use_id: impl Into<String>,
    ) -> Self {
        Self {
            working_dir,
            session_id: session_id.into(),
            tool_use_id: tool_use_id.into(),
        }
    }
}

/// Tool error.
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),

    #[error("Permission denied: {0}")]
    Permission(String),

    #[error("Execution failed: {0}")]
    Execution(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}
