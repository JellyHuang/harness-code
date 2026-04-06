//! Anthropic API types.

use hcode_types::{Message, ToolDefinition};
use serde::{Deserialize, Serialize};

/// Anthropic message request.
#[derive(Debug, Serialize)]
pub struct MessagesRequest {
    pub model: String,
    pub messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<AnthropicToolDefinition>>,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,
    pub stream: bool,
}

/// Anthropic message.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnthropicMessage {
    pub role: AnthropicRole,
    pub content: Vec<AnthropicContent>,
}

impl From<Message> for AnthropicMessage {
    fn from(msg: Message) -> Self {
        match msg {
            Message::User(user_msg) => Self {
                role: AnthropicRole::User,
                content: user_msg
                    .message
                    .content
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            },
            Message::Assistant(assistant_msg) => Self {
                role: AnthropicRole::Assistant,
                content: assistant_msg
                    .message
                    .content
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            },
            // For other message types, skip or convert appropriately
            _ => Self {
                role: AnthropicRole::User,
                content: vec![],
            },
        }
    }
}

/// Anthropic role.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AnthropicRole {
    User,
    Assistant,
}

/// Anthropic content block.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AnthropicContent {
    Text {
        text: String,
    },
    Thinking {
        thinking: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
}

impl From<hcode_types::ContentBlock> for AnthropicContent {
    fn from(content: hcode_types::ContentBlock) -> Self {
        match content {
            hcode_types::ContentBlock::Text { text } => Self::Text { text },
            hcode_types::ContentBlock::ToolUse { id, name, input } => {
                Self::ToolUse { id, name, input }
            }
            hcode_types::ContentBlock::ToolResult {
                tool_use_id,
                content,
                is_error,
            } => Self::ToolResult {
                tool_use_id,
                content,
                is_error: Some(is_error),
            },
            hcode_types::ContentBlock::Thinking { thinking } => Self::Thinking { thinking },
            hcode_types::ContentBlock::RedactedThinking { .. } => Self::Text {
                text: String::new(),
            },
            hcode_types::ContentBlock::Image { .. } => Self::Text {
                // Images not yet supported in this conversion
                text: "[Image content]".to_string(),
            },
        }
    }
}

/// Thinking configuration.
#[derive(Debug, Serialize)]
pub struct ThinkingConfig {
    pub thinking_type: String,
    pub budget_tokens: u32,
}

/// Anthropic tool definition.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnthropicToolDefinition {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

impl From<ToolDefinition> for AnthropicToolDefinition {
    fn from(tool: ToolDefinition) -> Self {
        Self {
            name: tool.name,
            description: Some(tool.description),
            input_schema: tool.parameters,
        }
    }
}

/// SSE event from Anthropic API.
#[derive(Debug, Deserialize)]
pub struct SseEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(flatten)]
    pub data: serde_json::Value,
}

/// Anthropic streaming response events.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: MessageStart },
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: usize,
        content_block: AnthropicContentBlock,
    },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: usize, delta: ContentDelta },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: usize },
    #[serde(rename = "message_delta")]
    MessageDelta { delta: MessageDelta, usage: Usage },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "error")]
    Error { error: ApiError },
}

#[derive(Debug, Deserialize)]
pub struct MessageStart {
    pub id: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub role: String,
    pub content: Vec<serde_json::Value>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: Usage,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum AnthropicContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "thinking")]
    Thinking { thinking: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ContentDelta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    #[serde(rename = "thinking_delta")]
    ThinkingDelta { thinking: String },
    #[serde(rename = "input_json_delta")]
    InputJsonDelta { partial_json: String },
    #[serde(rename = "signature_delta")]
    SignatureDelta { signature: String },
}

#[derive(Debug, Deserialize, Default)]
pub struct MessageDelta {
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct ApiError {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}
