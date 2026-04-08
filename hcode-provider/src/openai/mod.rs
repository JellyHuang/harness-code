//! OpenAI provider module.

mod types;
mod client;

pub use types::*;
pub use client::*;

use crate::{Provider, ProviderError, CompletionResponse};
use async_trait::async_trait;
use hcode_types::Message;
use std::pin::Pin;
use futures::Stream;

/// OpenAI provider wrapper.
pub struct OpenAIProvider {
    client: client::OpenAIClient,
}

impl OpenAIProvider {
    pub fn new(config: OpenAIConfig) -> Result<Self, ProviderError> {
        let client = client::OpenAIClient::new(config)?;
        Ok(Self { client })
    }
}

#[async_trait]
impl Provider for OpenAIProvider {
    fn name(&self) -> &str {
        "openai"
    }

    fn model(&self) -> &str {
        self.client.model()
    }

    async fn stream(
        &self,
        messages: Vec<Message>,
        tools: Vec<hcode_types::ToolDefinition>,
        system_prompt: Option<String>,
    ) -> Result<Pin<Box<dyn Stream<Item = hcode_protocol::StreamEvent> + Send>>, ProviderError> {
        self.client.stream(messages, tools, system_prompt).await
    }

    async fn complete(&self, messages: Vec<Message>, tools: Vec<hcode_types::ToolDefinition>) -> Result<CompletionResponse, ProviderError> {
        self.client.complete(messages, tools).await
    }
}