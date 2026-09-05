//! Bash tool implementation.

mod executor;
mod sandbox;
mod schema;

use crate::{Tool, ToolContext, ToolError};
use async_trait::async_trait;
use hcode_types::ToolResult;
pub use schema::*;
use serde_json::Value;

/// Bash tool for executing shell commands.
pub struct BashTool;

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Execute a bash command and return its output"
    }

    fn input_schema(&self) -> &Value {
        &BASH_SCHEMA
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, context: ToolContext) -> Result<ToolResult, ToolError> {
        let params: BashInput =
            serde_json::from_value(input).map_err(|e| ToolError::InvalidInput(e.to_string()))?;

        executor::execute(params, context).await
    }
}
