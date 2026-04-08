//! Input schema for Bash tool.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::LazyLock;

/// Default timeout in milliseconds (2 minutes).
pub const DEFAULT_TIMEOUT_MS: u64 = 120_000;

/// Maximum timeout in milliseconds (10 minutes).
pub const MAX_TIMEOUT_MS: u64 = 600_000;

/// Bash tool input parameters.
#[derive(Debug, Deserialize, Serialize)]
pub struct BashInput {
    /// The command to execute.
    pub command: String,
    
    /// Working directory for the command.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workdir: Option<String>,
    
    /// Timeout in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    
    /// Run in background (don't wait for completion).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_in_background: Option<bool>,
}

/// Bash tool output.
#[derive(Debug, Deserialize, Serialize)]
pub struct BashOutput {
    /// stdout content.
    pub stdout: String,
    
    /// stderr content.
    pub stderr: String,
    
    /// Exit code.
    pub exit_code: i32,
    
    /// Whether the command timed out.
    pub timed_out: bool,
}

/// JSON schema for Bash tool input.
pub static BASH_SCHEMA: LazyLock<Value> = LazyLock::new(|| json!({
    "type": "object",
    "properties": {
        "command": {
            "type": "string",
            "description": "The command to execute"
        },
        "workdir": {
            "type": "string",
            "description": "Working directory for the command"
        },
        "timeout": {
            "type": "number",
            "description": "Timeout in milliseconds (max 600000)",
            "minimum": 1000,
            "maximum": MAX_TIMEOUT_MS
        },
        "run_in_background": {
            "type": "boolean",
            "description": "Run in background without waiting"
        }
    },
    "required": ["command"]
}));