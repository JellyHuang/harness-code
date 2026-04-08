//! Hook events.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Hook event types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HookEvent {
    /// Before a tool is executed.
    PreToolUse {
        tool: String,
        input: Value,
    },
    
    /// After a tool is executed.
    PostToolUse {
        tool: String,
        result: String,
        is_error: bool,
    },
    
    /// Before a query is sent.
    PreQuery {
        prompt: String,
    },
    
    /// After a query completes.
    PostQuery {
        response: String,
    },
    
    /// Session started.
    SessionStart,
    
    /// Session ended.
    SessionEnd,
    
    /// Error occurred.
    Error {
        error: String,
    },
}

impl HookEvent {
    /// Get the event name.
    pub fn event_name(&self) -> &'static str {
        match self {
            HookEvent::PreToolUse { .. } => "pre_tool_use",
            HookEvent::PostToolUse { .. } => "post_tool_use",
            HookEvent::PreQuery { .. } => "pre_query",
            HookEvent::PostQuery { .. } => "post_query",
            HookEvent::SessionStart => "session_start",
            HookEvent::SessionEnd => "session_end",
            HookEvent::Error { .. } => "error",
        }
    }
}

/// Hook configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HookConfig {
    /// Event to hook into.
    pub event: String,
    
    /// Command to execute.
    pub command: String,
    
    /// Timeout in milliseconds.
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    
    /// Whether the hook is enabled.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_timeout() -> u64 {
    30000
}

fn default_enabled() -> bool {
    true
}

/// Hook execution result.
#[derive(Debug, Clone)]
pub struct HookResult {
    /// Whether the hook succeeded.
    pub success: bool,
    
    /// Output from the hook.
    pub output: Option<String>,
    
    /// Error message if failed.
    pub error: Option<String>,
    
    /// Whether to block the original action (for pre-hooks).
    pub block: bool,
}