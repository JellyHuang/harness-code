//! OpenAI provider module.

mod types;
mod client;

pub use types::*;
pub use client::*;

use crate::{Provider, ProviderError};
use async_trait::async_trait;
use hcode_types::Message;

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
    async fn complete(&self, messages: Vec<Message>, tools: Vec<hcode_types::ToolDefinition>) -> Result<Message, ProviderError> {
        self.client.complete(messages, tools).await
    }
    
    fn name(&self) -> &str {
        "openai"
    }
}