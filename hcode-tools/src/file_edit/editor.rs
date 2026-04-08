//! File editing implementation.

use super::schema::{EditInput, EditOutput};
use crate::{ToolContext, ToolError};
use hcode_types::ToolResult;
use std::path::Path;
use tokio::fs;

/// Edit a file by replacing text.
pub async fn edit_file(input: EditInput, context: ToolContext) -> Result<ToolResult, ToolError> {
    let path = Path::new(&input.file_path);
    
    let full_path = if path.is_relative() {
        context.working_dir.join(path)
    } else {
        path.to_path_buf()
    };

    // Read current content
    let content = fs::read_to_string(&full_path).await
        .map_err(|e| ToolError::Execution(format!("Failed to read file: {}", e)))?;

    // Check if old_string exists
    if !content.contains(&input.old_string) {
        return Err(ToolError::Execution("Text not found in file".to_string()));
    }

    // Perform replacement
    let new_content = if input.replace_all {
        content.replace(&input.old_string, &input.new_string)
    } else {
        // Replace only first occurrence
        if let Some(pos) = content.find(&input.old_string) {
            let mut result = content.clone();
            result.replace_range(pos..pos + input.old_string.len(), &input.new_string);
            result
        } else {
            return Err(ToolError::Execution("Text not found in file".to_string()));
        }
    };

    // Count replacements
    let count = if input.replace_all {
        content.matches(&input.old_string).count()
    } else {
        1
    };

    // Write back
    fs::write(&full_path, &new_content).await
        .map_err(|e| ToolError::Execution(format!("Failed to write file: {}", e)))?;

    Ok(ToolResult::success(
        serde_json::to_value(EditOutput {
            file_path: input.file_path,
            replacements: count,
        }).unwrap()
    ))
}