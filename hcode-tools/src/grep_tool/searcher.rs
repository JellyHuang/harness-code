//! Grep content search implementation.

use super::schema::{GrepInput, GrepMatch, GrepOutput, DEFAULT_LIMIT, MAX_LIMIT};
use crate::{ToolContext, ToolError};
use hcode_types::ToolResult;
use regex::Regex;
use std::path::Path;

/// Search file contents for a pattern.
pub async fn search_content(input: GrepInput, context: ToolContext) -> Result<ToolResult, ToolError> {
    // Validate and compile regex
    let pattern = Regex::new(&input.pattern)
        .map_err(|e| ToolError::InvalidInput(format!("Invalid regex: {}", e)))?;
    
    let limit = input.limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);
    
    let base_path = Path::new(&input.path.unwrap_or_else(|| ".".to_string()));
    
    let full_base = if base_path.is_relative() {
        context.working_dir.join(base_path)
    } else {
        base_path.to_path_buf()
    };

    // Run search in blocking thread
    let matches: Vec<GrepMatch> = tokio::task::spawn_blocking(move || {
        use walkdir::WalkDir;
        use std::fs;
        
        let mut results: Vec<GrepMatch> = Vec::new();
        
        for entry in WalkDir::new(&full_base)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            if results.len() >= limit {
                break;
            }

            let path = entry.path();
            
            // Skip binary files (try to read as string)
            let content = fs::read_to_string(path);
            if content.is_err() {
                continue;
            }

            for (line_num, line) in content.unwrap().lines().enumerate() {
                if pattern.is_match(line) {
                    results.push(GrepMatch {
                        file: path.to_str().unwrap().to_string(),
                        line: line_num + 1,
                        content: line.to_string(),
                    });
                    
                    if results.len() >= limit {
                        break;
                    }
                }
            }
        }
        
        results
    }).await.unwrap();

    Ok(ToolResult::success(
        serde_json::to_value(GrepOutput {
            matches: matches.clone(),
            count: matches.len(),
        }).unwrap()
    ))
}