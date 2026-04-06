//! Message types for LLM communication.
//!
//! Extended to match TypeScript QueryEngine message types from cc-haha.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Message UUID type alias
pub type MessageId = String;

/// Message types matching TypeScript's Message union type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    /// User message (can contain tool results)
    User(UserMessage),
    /// Assistant message with content blocks
    Assistant(AssistantMessage),
    /// System message (informational, compact boundary, etc.)
    System(SystemMessage),
    /// Progress message for streaming updates
    Progress(ProgressMessage),
    /// Attachment message for files, images, etc.
    Attachment(AttachmentMessage),
    /// Tombstone for removed messages (control signal)
    Tombstone(TombstoneMessage),
    /// Stream event for API streaming
    StreamEvent(StreamEventMessage),
    /// Tool use summary
    ToolUseSummary(ToolUseSummaryMessage),
}

/// User message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub uuid: MessageId,
    pub timestamp: DateTime<Utc>,
    pub message: UserMessageContent,
    #[serde(default)]
    pub is_meta: bool,
    #[serde(default)]
    pub tool_use_result: Option<String>,
    #[serde(default)]
    pub image_paste_ids: Option<Vec<u64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessageContent {
    pub role: Role,
    pub content: Vec<ContentBlock>,
}

/// Assistant message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub uuid: MessageId,
    pub timestamp: DateTime<Utc>,
    pub message: AssistantMessageContent,
    #[serde(default)]
    pub is_api_error_message: Option<bool>,
    #[serde(default)]
    pub api_error: Option<String>,
    #[serde(default)]
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessageContent {
    pub id: String,
    pub role: Role,
    pub model: String,
    pub content: Vec<ContentBlock>,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: Option<Usage>,
}

/// System message subtypes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub enum SystemMessage {
    /// Compact boundary marker
    CompactBoundary {
        uuid: MessageId,
        timestamp: DateTime<Utc>,
        compact_metadata: Option<CompactMetadata>,
    },
    /// API error/retry notification
    ApiError {
        uuid: MessageId,
        timestamp: DateTime<Utc>,
        error: ApiErrorInfo,
        retry_attempt: Option<u32>,
        max_retries: Option<u32>,
        retry_in_ms: Option<u64>,
    },
    /// Informational message
    Informational {
        uuid: MessageId,
        timestamp: DateTime<Utc>,
        content: String,
        level: SystemMessageLevel,
    },
    /// Local command output
    LocalCommand {
        uuid: MessageId,
        timestamp: DateTime<Utc>,
        content: String,
    },
}

/// Progress message for streaming/tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressMessage {
    pub uuid: MessageId,
    pub timestamp: DateTime<Utc>,
    pub tool_use_id: String,
    pub parent_tool_use_id: Option<String>,
    pub data: ProgressData,
}

/// Progress data variants
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProgressData {
    BashProgress {
        elapsed_time_seconds: f64,
        task_id: Option<String>,
    },
    AgentProgress {
        message: Box<Message>,
    },
    SkillProgress {
        message: Box<Message>,
    },
    HookProgress {
        command: String,
        prompt_text: Option<String>,
    },
}

/// Attachment message for files, images, structured output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentMessage {
    pub uuid: MessageId,
    pub timestamp: DateTime<Utc>,
    pub attachment: Attachment,
}

/// Attachment variants
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Attachment {
    /// File was edited
    EditedTextFile {
        file_path: String,
        diff: Option<String>,
    },
    /// Structured output from tool
    StructuredOutput { data: serde_json::Value },
    /// Max turns reached signal
    MaxTurnsReached { max_turns: u32, turn_count: u32 },
    /// Queued command
    QueuedCommand {
        prompt: String,
        source_uuid: Option<String>,
    },
    /// Hook stopped continuation
    HookStoppedContinuation {
        message: String,
        hook_name: String,
        tool_use_id: String,
        hook_event: String,
    },
    /// Memory attachment
    Memory { file_path: String, content: String },
}

/// Tombstone message (control signal for removing messages)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TombstoneMessage {
    pub message: Box<Message>,
}

/// Stream event from API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEventMessage {
    pub uuid: MessageId,
    pub event: StreamEvent,
}

/// Stream event variants
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    MessageStart {
        message: MessageStartData,
    },
    ContentBlockStart {
        index: u32,
        content_block: ContentBlock,
    },
    ContentBlockDelta {
        index: u32,
        delta: ContentDelta,
    },
    ContentBlockStop {
        index: u32,
    },
    MessageDelta {
        delta: MessageDelta,
        usage: Option<Usage>,
    },
    MessageStop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageStartData {
    pub id: String,
    pub model: String,
    pub role: Role,
    pub usage: Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentDelta {
    TextDelta { text: String },
    ThinkingDelta { thinking: String },
    InputJsonDelta { partial_json: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDelta {
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
}

/// Tool use summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUseSummaryMessage {
    pub uuid: MessageId,
    pub summary: String,
    pub preceding_tool_use_ids: Vec<String>,
}

/// The role of a message sender
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
}

/// A content block in a message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Text content
    Text { text: String },
    /// Thinking content (from Claude)
    Thinking { thinking: String },
    /// Redacted thinking content
    RedactedThinking { data: String },
    /// A tool use request
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    /// A tool use result
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(default)]
        is_error: bool,
    },
    /// Image content
    Image { source: ImageSource },
}

/// Image source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub media_type: String,
    pub data: String,
}

/// Token usage information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
}

/// Compact metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactMetadata {
    pub pre_compact_token_count: u64,
    pub post_compact_token_count: u64,
    pub summary_messages: Vec<Message>,
}

/// API error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErrorInfo {
    pub status: Option<u16>,
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
}

/// System message level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SystemMessageLevel {
    Info,
    Warning,
    Error,
}

/// Builder for creating messages easily
impl Message {
    /// Create a simple user text message
    pub fn user_text(text: impl Into<String>) -> Self {
        Message::User(UserMessage {
            uuid: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            message: UserMessageContent {
                role: Role::User,
                content: vec![ContentBlock::Text { text: text.into() }],
            },
            is_meta: false,
            tool_use_result: None,
            image_paste_ids: None,
        })
    }

    /// Create a simple assistant text message
    pub fn assistant_text(text: impl Into<String>) -> Self {
        Message::Assistant(AssistantMessage {
            uuid: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            message: AssistantMessageContent {
                id: Uuid::new_v4().to_string(),
                role: Role::Assistant,
                model: String::new(),
                content: vec![ContentBlock::Text { text: text.into() }],
                stop_reason: None,
                stop_sequence: None,
                usage: None,
            },
            is_api_error_message: None,
            api_error: None,
            request_id: None,
        })
    }
}

impl ContentBlock {
    /// Create a text content block
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    /// Create a tool use content block
    pub fn tool_use(
        id: impl Into<String>,
        name: impl Into<String>,
        input: serde_json::Value,
    ) -> Self {
        Self::ToolUse {
            id: id.into(),
            name: name.into(),
            input,
        }
    }

    /// Create a tool result content block
    pub fn tool_result(
        tool_use_id: impl Into<String>,
        content: impl Into<String>,
        is_error: bool,
    ) -> Self {
        Self::ToolResult {
            tool_use_id: tool_use_id.into(),
            content: content.into(),
            is_error,
        }
    }
}
