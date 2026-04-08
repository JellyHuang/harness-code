//! Input schema for FileEdit tool.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Edit tool input parameters.
#[derive(Debug, Deserialize)]
pub struct EditInput {
    /// Absolute path to edit.
    pub file_path: String,
    
    /// Text to find and replace.
    pub old_string: String,
    
    /// Text to replace with.
    pub new_string: String,
    
    /// Replace all occurrences.
    #[serde(default)]
    pub replace_all: bool,
}

/// Edit tool output.
#[derive(Debug, Serialize)]
pub struct EditOutput {
    /// File path that was edited.
    pub file_path: String,
    
    /// Number of replacements made.
    pub replacements: usize,
}

/// JSON schema for Edit tool input.
pub static EDIT_SCHEMA: Value = json!({
    "type": "object",
    "properties": {
        "file_path": {
            "type": "string",
            "description": "The absolute path to edit"
        },
        "old_string": {
            "type": "string",
            "description": "The text to find and replace"
        },
        "new_string": {
            "type": "string",
            "description": "The text to replace with"
        },
        "replace_all": {
            "type": "boolean",
            "description": "Replace all occurrences",
            "default": false
        }
    },
    "required": ["file_path", "old_string", "new_string"]
});