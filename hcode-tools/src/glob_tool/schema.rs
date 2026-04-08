//! Input schema for Glob tool.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Default result limit.
pub const DEFAULT_LIMIT: usize = 100;

/// Maximum result limit.
pub const MAX_LIMIT: usize = 1000;

/// Glob tool input parameters.
#[derive(Debug, Deserialize)]
pub struct GlobInput {
    /// Glob pattern to match files.
    pub pattern: String,
    
    /// Base directory to search in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    
    /// Maximum number of results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

/// Glob tool output.
#[derive(Debug, Serialize)]
pub struct GlobOutput {
    /// List of matching file paths.
    pub files: Vec<String>,
    
    /// Number of files found.
    pub count: usize,
}

/// JSON schema for Glob tool input.
pub static GLOB_SCHEMA: Value = json!({
    "type": "object",
    "properties": {
        "pattern": {
            "type": "string",
            "description": "Glob pattern to match files (e.g., '**/*.js')"
        },
        "path": {
            "type": "string",
            "description": "Base directory to search in (defaults to current directory)"
        },
        "limit": {
            "type": "number",
            "description": "Maximum number of results",
            "minimum": 1,
            "maximum": MAX_LIMIT,
            "default": DEFAULT_LIMIT
        }
    },
    "required": ["pattern"]
});