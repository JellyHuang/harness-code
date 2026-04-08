//! TaskStopTool for stopping background tasks.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::LazyLock;

/// TaskStop input parameters.
#[derive(Debug, Deserialize)]
pub struct TaskStopInput {
    /// Task/worker ID to stop.
    pub task_id: String,
}

/// TaskStop result.
#[derive(Debug, Serialize)]
pub struct TaskStopResult {
    /// Task ID.
    pub task_id: String,
    
    /// Whether the task was stopped.
    pub stopped: bool,
    
    /// Message.
    pub message: String,
}

/// JSON schema for TaskStop tool.
pub static TASK_STOP_SCHEMA: LazyLock<Value> = LazyLock::new(|| json!({
    "type": "object",
    "properties": {
        "task_id": {
            "type": "string",
            "description": "The task/worker ID to stop"
        }
    },
    "required": ["task_id"]
}));