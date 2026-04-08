//! SendMessageTool for sending messages to coordinator/workers.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// SendMessage input parameters.
#[derive(Debug, Deserialize)]
pub struct SendMessageInput {
    /// Message content.
    pub message: String,
    
    /// Target worker ID (None = broadcast to all).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

/// SendMessage result.
#[derive(Debug, Serialize)]
pub struct SendMessageResult {
    /// Whether the message was sent.
    pub sent: bool,
    
    /// Message.
    pub message: String,
    
    /// Number of recipients.
    pub recipients: usize,
}

/// JSON schema for SendMessage tool.
pub static SEND_MESSAGE_SCHEMA: Value = json!({
    "type": "object",
    "properties": {
        "message": {
            "type": "string",
            "description": "The message to send"
        },
        "target": {
            "type": "string",
            "description": "Target worker ID (omit to broadcast to all)"
        }
    },
    "required": ["message"]
});