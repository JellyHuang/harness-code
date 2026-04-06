//! Stream event types for SSE and provider communication.

/// Events from LLM streaming.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// Message started.
    MessageStart { id: String, model: String },
    /// Content block started.
    ContentBlockStart {
        index: usize,
        block_type: ContentBlockType,
    },
    /// Content block delta.
    ContentBlockDelta { index: usize, delta: ContentDelta },
    /// Content block stopped.
    ContentBlockStop { index: usize },
    /// Message delta with stop reason.
    MessageDelta {
        stop_reason: Option<String>,
        usage: StreamUsage,
    },
    /// Message stopped.
    MessageStop,
    /// Error occurred.
    Error { message: String },
}

/// Type of content block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentBlockType {
    Text,
    Thinking,
    ToolUse,
}

/// Delta content in a stream.
#[derive(Debug, Clone)]
pub enum ContentDelta {
    /// Text delta.
    Text { text: String },
    /// Thinking delta.
    Thinking { thinking: String },
    /// Tool input JSON delta.
    InputJsonDelta { partial_json: String },
}

/// Usage information in a stream.
#[derive(Debug, Clone, Default)]
pub struct StreamUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_tokens: Option<u32>,
    pub cache_write_tokens: Option<u32>,
}
