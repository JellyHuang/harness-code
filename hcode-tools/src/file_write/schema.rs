//! Input schema for FileWrite tool.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Write tool input parameters.
#[derive(Debug, Deserialize)]
pub struct WriteInput {
    /// Absolute path to write to.
    pub file_path: String,
    
    /// Content to write.
    pub content: String,
}

/// Write tool output.
#[derive(Debug, Serialize)]
pub struct WriteOutput {
    /// File path that was written.
    pub file_path: String,
    
    /// Number of bytes written.
    pub bytes_written: usize,
}

/// JSON schema for Write tool input.
pub static WRITE_SCHEMA: Value = json!({
    "type": "object",
    "properties": {
        "file_path": {
            "type": "string",
            "description": "The absolute path to write to"
        },
        "content": {
            "type": "string",
            "description": "The content to write"
        }
    },
    "required": ["file_path", "content"]
});