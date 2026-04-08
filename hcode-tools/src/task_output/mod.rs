//! TaskOutputTool implementation.

mod schema;

use crate::{Tool, ToolContext, ToolError};
use async_trait::async_trait;
use hcode_types::ToolResult;
use serde_json::Value;
pub use schema::*;

/// TaskOutput tool for getting background task output.
pub struct TaskOutputTool;

#[async_trait]
impl Tool for TaskOutputTool {
    fn name(&self) -> &str {
        "task_output"
    }

    fn description(&self) -> &str {
        "Get the output from a background task/agent"
    }

    fn input_schema(&self) -> &Value {
        &TASK_OUTPUT_SCHEMA
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _context: ToolContext) -> Result<ToolResult, ToolError> {
        let params: TaskOutputInput = serde_json::from_value(input)
            .map_err(|e| ToolError::InvalidInput(e.to_string()))?;

        // Note: Full coordinator integration requires QueryEngine setup
        // This placeholder returns not_found until coordinator is wired up
        Ok(ToolResult::success(
            serde_json::to_value(TaskOutputResult {
                task_id: params.task_id,
                status: "not_found".to_string(),
                result: None,
                error: Some("Task not found or coordinator not available".to_string()),
            }).unwrap()
        ))
    }
}