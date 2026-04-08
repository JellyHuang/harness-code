//! Input schema for Grep tool.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::LazyLock;

/// Default result limit.
pub const DEFAULT_LIMIT: usize = 100;

/// Maximum result limit.
pub const MAX_LIMIT: usize = 1000;

/// Maximum output size (256KB).
pub const MAX_OUTPUT_SIZE: usize = 256 * 1024;

/// Grep tool input parameters.
#[derive(Debug, Deserialize)]
pub struct GrepInput {
    /// Regex pattern to search for.
    pub pattern: String,
    
    /// Base directory to search in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    
    /// File pattern to filter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include: Option<String>,
    
    /// Maximum number of results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    
    /// Output mode: "content", "files_with_matches", or "count".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_mode: Option<String>,
}

/// A single grep match.
#[derive(Debug, Clone, Serialize)]
pub struct GrepMatch {
    /// File path.
    pub file: String,
    
    /// Line number (1-indexed).
    pub line: usize,
    
    /// Content of the matching line.
    pub content: String,
}

/// Grep tool output.
#[derive(Debug, Serialize)]
pub struct GrepOutput {
    /// List of matches.
    pub matches: Vec<GrepMatch>,
    
    /// Number of matches found.
    pub count: usize,
}

/// JSON schema for Grep tool input.
pub static GREP_SCHEMA: LazyLock<Value> = LazyLock::new(|| json!({
    "type": "object",
    "properties": {
        "pattern": {
            "type": "string",
            "description": "Regex pattern to search for"
        },
        "path": {
            "type": "string",
            "description": "Base directory to search in"
        },
        "include": {
            "type": "string",
            "description": "File pattern to filter (e.g., '*.js')"
        },
        "limit": {
            "type": "number",
            "description": "Maximum number of results",
            "minimum": 1,
            "maximum": MAX_LIMIT
        },
        "output_mode": {
            "type": "string",
            "enum": ["content", "files_with_matches", "count"],
            "description": "Output format mode",
            "default": "content"
        }
    },
    "required": ["pattern"]
}));