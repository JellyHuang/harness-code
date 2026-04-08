//! Anthropic SSE stream parser.

use crate::anthropic::types::AnthropicContentBlock;
use bytes::Bytes;
use futures::{Stream, StreamExt};
use hcode_protocol::{ContentBlockType, ContentDelta, StreamEvent, StreamUsage};
use reqwest::Response;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Anthropic streaming response parser.
pub struct AnthropicStream {
    inner: Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>>,
    buffer: String,
    message_id: Option<String>,
    model: Option<String>,
    stop_reason: Option<String>,
    usage: StreamUsage,
    current_event_type: Option<String>,
}

impl AnthropicStream {
    /// Create a new Anthropic stream from an HTTP response.
    pub fn new(response: Response) -> Self {
        let stream = response.bytes_stream();
        Self {
            inner: Box::pin(stream),
            buffer: String::new(),
            message_id: None,
            model: None,
            stop_reason: None,
            usage: StreamUsage::default(),
            current_event_type: None,
        }
    }

    /// Parse an SSE line.
    fn parse_line(&mut self, line: &str) -> Option<StreamEvent> {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }

        // Handle event: line
        if line.starts_with("event:") {
            self.current_event_type = Some(line[6..].trim().to_string());
            return None;
        }

        // Handle data: line
        if !line.starts_with("data:") {
            return None;
        }

        let data = line[5..].trim();
        if data == "[DONE]" {
            return Some(StreamEvent::MessageStop);
        }

        // Parse JSON
        let event: crate::anthropic::types::StreamEvent = match serde_json::from_str(data) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("[SSE] Failed to parse JSON: {} - {}", e, data);
                return Some(StreamEvent::Error {
                    message: format!("Failed to parse SSE event: {}", data),
                });
            }
        };

        match event {
            crate::anthropic::types::StreamEvent::MessageStart { message } => {
                self.message_id = Some(message.id.clone());
                self.model = Some(message.model.clone());
                self.usage.input_tokens = message.usage.input_tokens;
                Some(StreamEvent::MessageStart {
                    id: message.id,
                    model: message.model,
                })
            }
            crate::anthropic::types::StreamEvent::ContentBlockStart {
                index,
                content_block,
            } => {
                let (block_type, tool_id, tool_name) = match content_block {
                    AnthropicContentBlock::Text { .. } => (ContentBlockType::Text, None, None),
                    AnthropicContentBlock::Thinking { .. } => (ContentBlockType::Thinking, None, None),
                    AnthropicContentBlock::ToolUse { id, name, .. } => {
                        (ContentBlockType::ToolUse, Some(id), Some(name))
                    }
                };
                Some(StreamEvent::ContentBlockStart { 
                    index, 
                    block_type,
                    tool_id,
                    tool_name,
                })
            }
            crate::anthropic::types::StreamEvent::ContentBlockDelta { index, delta } => {
                let content_delta = match delta {
                    crate::anthropic::types::ContentDelta::TextDelta { text } => {
                        ContentDelta::Text { text }
                    }
                    crate::anthropic::types::ContentDelta::ThinkingDelta { thinking } => {
                        ContentDelta::Thinking { thinking }
                    }
                    crate::anthropic::types::ContentDelta::InputJsonDelta { partial_json } => {
                        ContentDelta::InputJsonDelta { partial_json }
                    }
                    crate::anthropic::types::ContentDelta::SignatureDelta { .. } => {
                        // Signature delta is used for thinking signature, skip it
                        return None;
                    }
                };
                Some(StreamEvent::ContentBlockDelta {
                    index,
                    delta: content_delta,
                })
            }
            crate::anthropic::types::StreamEvent::ContentBlockStop { index } => {
                Some(StreamEvent::ContentBlockStop { index })
            }
            crate::anthropic::types::StreamEvent::MessageDelta { delta, usage } => {
                self.stop_reason = delta.stop_reason.clone();
                self.usage.output_tokens = usage.output_tokens;
                self.usage.input_tokens = usage.input_tokens;
                Some(StreamEvent::MessageDelta {
                    stop_reason: delta.stop_reason,
                    usage: StreamUsage {
                        input_tokens: usage.input_tokens,
                        output_tokens: usage.output_tokens,
                        cache_read_tokens: usage.cache_read_tokens,
                        cache_write_tokens: usage.cache_creation_tokens,
                    },
                })
            }
            crate::anthropic::types::StreamEvent::MessageStop => Some(StreamEvent::MessageStop),
            crate::anthropic::types::StreamEvent::Ping => None, // Ignore ping events
            crate::anthropic::types::StreamEvent::Error { error } => Some(StreamEvent::Error {
                message: format!("{}: {}", error.error_type, error.message),
            }),
        }
    }
}

impl Stream for AnthropicStream {
    type Item = StreamEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match self.inner.poll_next_unpin(cx) {
                Poll::Ready(Some(Ok(bytes))) => {
                    // Convert bytes to string and append to buffer
                    if let Ok(chunk) = std::str::from_utf8(&bytes) {
                        self.buffer.push_str(chunk);

                        // Process complete lines
                        while let Some(pos) = self.buffer.find('\n') {
                            let line = self.buffer[..pos].to_string();
                            self.buffer = self.buffer[pos + 1..].to_string();

                            if let Some(event) = self.parse_line(&line) {
                                return Poll::Ready(Some(event));
                            }
                        }
                    }
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(StreamEvent::Error {
                        message: format!("Stream error: {}", e),
                    }));
                }
                Poll::Ready(None) => {
                    // Process any remaining data in buffer
                    if !self.buffer.is_empty() {
                        let line = self.buffer.clone();
                        self.buffer.clear();
                        if let Some(event) = self.parse_line(&line) {
                            return Poll::Ready(Some(event));
                        }
                    }
                    return Poll::Ready(None);
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}
