//! File writing implementation.

use super::schema::{WriteInput, WriteOutput};
use crate::{ToolContext, ToolError};
use hcode_types::ToolResult;
use std::path::Path;
use tokio::fs;

/// Write content to a file.
pub async fn write_file(input: WriteInput, context: ToolContext) -> Result<ToolResult, ToolError> {
    let path = Path::new(&input.file_path);
    
    let full_path = if path.is_relative() {
        context.working_dir.join(path)
    } else {
        path.to_path_buf()
    };

    // Create parent directories if needed
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent).await
            .map_err(|e| ToolError::Execution(format!("Failed to create directory: {}", e)))?;
    }

    // Write file
    fs::write(&full_path, &input.content).await
        .map_err(|e| ToolError::Execution(format!("Failed to write file: {}", e)))?;

    Ok(ToolResult::success(
        serde_json::to_value(WriteOutput {
            file_path: input.file_path,
            bytes_written: input.content.len(),
        }).unwrap()
    ))
}