//! File read tool implementation.

mod reader;
mod schema;

use crate::{Tool, ToolContext, ToolError};
use async_trait::async_trait;
use hcode_types::ToolResult;
pub use schema::*;
use serde_json::Value;

/// File read tool.
pub struct FileReadTool;

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "read"
    }

    fn description(&self) -> &str {
        "Read a file from the local filesystem"
    }

    fn input_schema(&self) -> &Value {
        &READ_SCHEMA
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, context: ToolContext) -> Result<ToolResult, ToolError> {
        let params: ReadInput =
            serde_json::from_value(input).map_err(|e| ToolError::InvalidInput(e.to_string()))?;

        reader::read_file(params, context).await
    }
}
