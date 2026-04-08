//! File reading implementation.

use super::schema::{ReadInput, ReadOutput, MAX_FILE_SIZE};
use crate::{ToolContext, ToolError};
use hcode_types::ToolResult;
use std::path::Path;
use tokio::fs;

/// Read a file.
pub async fn read_file(input: ReadInput, context: ToolContext) -> Result<ToolResult, ToolError> {
    let path = Path::new(&input.file_path);
    
    // Resolve relative paths from working directory
    let full_path = if path.is_relative() {
        context.working_dir.join(path)
    } else {
        path.to_path_buf()
    };

    // Check file exists
    if !full_path.exists() {
        return Err(ToolError::Execution(
            format!("File does not exist: {}", input.file_path)
        ));
    }

    // Check file size
    let metadata = fs::metadata(&full_path).await
        .map_err(|e| ToolError::Execution(format!("Failed to read file metadata: {}", e)))?;
    
    if metadata.len() > MAX_FILE_SIZE as u64 {
        return Err(ToolError::Execution(
            format!("File too large: {} bytes (max {})", metadata.len(), MAX_FILE_SIZE)
        ));
    }

    // Read file content
    let content = fs::read_to_string(&full_path).await
        .map_err(|e| ToolError::Execution(format!("Failed to read file: {}", e)))?;

    // Split into lines
    let all_lines: Vec<&str> = content.lines().collect();
    let total_lines = all_lines.len();

    // Apply offset and limit
    let offset = input.offset.unwrap_or(1);
    let offset_idx = (offset - 1).min(total_lines);
    
    let limit = input.limit.unwrap_or(total_lines - offset_idx);
    let end_idx = (offset_idx + limit).min(total_lines);
    
    let selected_lines = &all_lines[offset_idx..end_idx];
    
    // Add line numbers
    let numbered_content = selected_lines
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{}: {}", offset_idx + i + 1, line))
        .collect::<Vec<_>>()
        .join("\n");

    Ok(ToolResult::success(
        serde_json::to_value(ReadOutput {
            file_path: input.file_path,
            content: numbered_content,
            num_lines: selected_lines.len(),
            start_line: offset,
            total_lines,
        }).unwrap()
    ))
}