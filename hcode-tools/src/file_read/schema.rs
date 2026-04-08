//! Input schema for FileRead tool.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Maximum file size to read (50MB).
pub const MAX_FILE_SIZE: usize = 50_000_000;

/// Maximum tokens to return (50K).
pub const MAX_TOKENS: usize = 50_000;

/// Read tool input parameters.
#[derive(Debug, Deserialize, Serialize)]
pub struct ReadInput {
    /// Absolute path to the file.
    pub file_path: String,
    
    /// Line number to start reading from (1-indexed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
    
    /// Number of lines to read.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

/// Read tool output.
#[derive(Debug, Serialize)]
pub struct ReadOutput {
    /// File path that was read.
    pub file_path: String,
    
    /// Content of the file (with line numbers).
    pub content: String,
    
    /// Number of lines returned.
    pub num_lines: usize,
    
    /// Starting line number.
    pub start_line: usize,
    
    /// Total lines in file.
    pub total_lines: usize,
}

/// JSON schema for Read tool input.
pub static READ_SCHEMA: Value = json!({
    "type": "object",
    "properties": {
        "file_path": {
            "type": "string",
            "description": "The absolute path to the file to read"
        },
        "offset": {
            "type": "number",
            "description": "The line number to start reading from (1-indexed)",
            "minimum": 1
        },
        "limit": {
            "type": "number",
            "description": "Number of lines to read",
            "minimum": 1
        }
    },
    "required": ["file_path"]
});