//! File edit tool implementation.

mod editor;
mod schema;

use crate::{Tool, ToolContext, ToolError};
use async_trait::async_trait;
use hcode_types::ToolResult;
pub use schema::*;
use serde_json::Value;

/// File edit tool for diff-based editing.
pub struct FileEditTool;

#[async_trait]
impl Tool for FileEditTool {
    fn name(&self) -> &str {
        "edit"
    }

    fn description(&self) -> &str {
        "Edit a file by replacing specific text"
    }

    fn input_schema(&self) -> &Value {
        &EDIT_SCHEMA
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, context: ToolContext) -> Result<ToolResult, ToolError> {
        let params: EditInput =
            serde_json::from_value(input).map_err(|e| ToolError::InvalidInput(e.to_string()))?;

        editor::edit_file(params, context).await
    }
}
