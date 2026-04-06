//! Tool-related types.

use serde::{Deserialize, Serialize};

/// A tool use request from the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUse {
    /// Unique identifier for this tool use.
    pub id: String,
    /// Name of the tool to use.
    pub name: String,
    /// Input parameters for the tool.
    pub input: serde_json::Value,
}

/// Result from a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// The content of the result.
    pub content: String,
    /// Whether this result represents an error.
    #[serde(default)]
    pub is_error: bool,
    /// Optional images in the result.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<ImageContent>,
}

impl ToolResult {
    /// Create a successful text result.
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            is_error: false,
            images: Vec::new(),
        }
    }

    /// Create an error result.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: message.into(),
            is_error: true,
            images: Vec::new(),
        }
    }
}

/// Image content in a tool result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageContent {
    /// The image data (base64 encoded).
    pub data: String,
    /// The media type of the image.
    pub media_type: String,
}

/// Definition of a tool for the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Name of the tool.
    pub name: String,
    /// Description of what the tool does.
    pub description: String,
    /// JSON Schema for the input parameters.
    pub parameters: serde_json::Value,
}

impl ToolDefinition {
    /// Create a new tool definition.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
        }
    }
}
