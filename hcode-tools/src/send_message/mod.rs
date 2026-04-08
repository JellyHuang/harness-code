//! SendMessageTool implementation.

mod schema;

use crate::{Tool, ToolContext, ToolError};
use async_trait::async_trait;
use hcode_types::ToolResult;
use serde_json::Value;
pub use schema::*;

/// SendMessage tool for sending messages to coordinator/workers.
pub struct SendMessageTool;

#[async_trait]
impl Tool for SendMessageTool {
    fn name(&self) -> &str {
        "send_message"
    }

    fn description(&self) -> &str {
        "Send a message to the coordinator or specific workers"
    }

    fn input_schema(&self) -> &Value {
        &SEND_MESSAGE_SCHEMA
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _context: ToolContext) -> Result<ToolResult, ToolError> {
        let params: SendMessageInput = serde_json::from_value(input)
            .map_err(|e| ToolError::InvalidInput(e.to_string()))?;

        // Note: Full coordinator integration requires QueryEngine setup
        Ok(ToolResult::success(
            serde_json::to_value(SendMessageResult {
                sent: false,
                message: params.message,
                recipients: 0,
            }).unwrap()
        ))
    }
}