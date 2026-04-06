//! Provider trait for LLM abstraction.

use async_trait::async_trait;
use futures::Stream;
use hcode_protocol::StreamEvent;
use hcode_types::{Message, ToolDefinition};
use serde::{Deserialize, Serialize};
use std::pin::Pin;

/// Provider trait for LLM API interaction.
#[async_trait]
pub trait Provider: Send + Sync {
    /// Get the provider name.
    fn name(&self) -> &str;

    /// Get the model name.
    fn model(&self) -> &str;

    /// Stream completion.
    async fn stream(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolDefinition>,
    ) -> Result<Pin<Box<dyn Stream<Item = StreamEvent> + Send>>, ProviderError>;

    /// Complete without streaming.
    async fn complete(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolDefinition>,
    ) -> Result<CompletionResponse, ProviderError>;
}

/// Completion response.
#[derive(Debug, Clone)]
pub struct CompletionResponse {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub usage: StreamUsage,
    pub stop_reason: String,
}

/// Tool call in a response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
}

/// Usage information.
#[derive(Debug, Clone, Default)]
pub struct StreamUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Provider error.
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("API error: {0}")]
    Api(String),

    #[error("Stream error: {0}")]
    Stream(String),

    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}
