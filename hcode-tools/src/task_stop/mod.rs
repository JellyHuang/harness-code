//! TaskStopTool implementation.

mod schema;

use crate::{Tool, ToolContext, ToolError};
use async_trait::async_trait;
use hcode_types::ToolResult;
use serde_json::Value;
pub use schema::*;

/// TaskStop tool for stopping background tasks.
pub struct TaskStopTool;

#[async_trait]
impl Tool for TaskStopTool {
    fn name(&self) -> &str {
        "task_stop"
    }

    fn description(&self) -> &str {
        "Stop a running background task/agent"
    }

    fn input_schema(&self) -> &Value {
        &TASK_STOP_SCHEMA
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, _context: ToolContext) -> Result<ToolResult, ToolError> {
        let params: TaskStopInput = serde_json::from_value(input)
            .map_err(|e| ToolError::InvalidInput(e.to_string()))?;

        // Note: Full coordinator integration requires QueryEngine setup
        Ok(ToolResult::success(
            serde_json::to_value(TaskStopResult {
                task_id: params.task_id,
                stopped: false,
                message: "Task not found or coordinator not available".to_string(),
            }).unwrap()
        ))
    }
}