//! TaskOutputTool for getting background task output.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::LazyLock;

/// TaskOutput input parameters.
#[derive(Debug, Deserialize)]
pub struct TaskOutputInput {
    /// Task/worker ID to get output from.
    pub task_id: String,
    
    /// Wait for completion if task is still running.
    #[serde(default)]
    pub wait: bool,
    
    /// Timeout in milliseconds for waiting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
}

/// TaskOutput result.
#[derive(Debug, Serialize)]
pub struct TaskOutputResult {
    /// Task ID.
    pub task_id: String,
    
    /// Current status.
    pub status: String,
    
    /// Result if completed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    
    /// Error if failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// JSON schema for TaskOutput tool.
pub static TASK_OUTPUT_SCHEMA: LazyLock<Value> = LazyLock::new(|| json!({
    "type": "object",
    "properties": {
        "task_id": {
            "type": "string",
            "description": "The task/worker ID to get output from"
        },
        "wait": {
            "type": "boolean",
            "description": "Wait for completion if task is still running",
            "default": false
        },
        "timeout": {
            "type": "number",
            "description": "Timeout in milliseconds for waiting (max 600000)",
            "maximum": 600000
        }
    },
    "required": ["task_id"]
}));