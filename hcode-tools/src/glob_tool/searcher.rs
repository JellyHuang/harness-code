//! Glob file search implementation.

use super::schema::{GlobInput, GlobOutput, DEFAULT_LIMIT, MAX_LIMIT};
use crate::{ToolContext, ToolError};
use hcode_types::ToolResult;
use std::path::Path;

/// Search files matching a glob pattern.
pub async fn search_files(input: GlobInput, context: ToolContext) -> Result<ToolResult, ToolError> {
    // Validate limit
    let limit = input.limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);
    
    let base_path = Path::new(&input.path.unwrap_or_else(|| ".".to_string()));
    
    let full_base = if base_path.is_relative() {
        context.working_dir.join(base_path)
    } else {
        base_path.to_path_buf()
    };

    // Build glob pattern
    let pattern = if input.pattern.starts_with('*') || input.pattern.contains('/') {
        // Pattern is already complete
        full_base.join(&input.pattern)
    } else {
        // Pattern is simple like "*.js", add to base
        full_base.join(&input.pattern)
    };
    
    let pattern_str = pattern.to_str().unwrap();

    // Execute glob search (blocking operation, run in spawn_blocking)
    let results: Vec<String> = tokio::task::spawn_blocking(move || {
        use glob::glob;
        
        glob(pattern_str)
            .unwrap_or_else(|e| panic!("Invalid glob pattern: {}", e))
            .filter_map(|entry| entry.ok())
            .take(limit)
            .map(|path| path.to_str().unwrap().to_string())
            .collect()
    }).await.unwrap();

    Ok(ToolResult::success(
        serde_json::to_value(GlobOutput {
            files: results.clone(),
            count: results.len(),
        }).unwrap()
    ))
}