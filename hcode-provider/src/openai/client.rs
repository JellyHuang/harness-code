//! OpenAI client implementation.

use super::types::*;
use crate::{Provider, ProviderConfig, ProviderError};
use async_trait::async_trait;
use hcode_types::{ContentBlock, Message, ToolUse, ToolResult};
use reqwest::Client;
use serde_json::json;
use std::sync::Arc;

/// OpenAI configuration.
#[derive(Debug, Clone)]
pub struct OpenAIConfig {
    pub api_key: String,
    pub base_url: Option<String>,
    pub model: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

impl Default for OpenAIConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: None,
            model: "gpt-4o".to_string(),
            temperature: Some(1.0),
            max_tokens: Some(4096),
        }
    }
}

/// OpenAI client.
pub struct OpenAIClient {
    config: OpenAIConfig,
    client: Client,
}

impl OpenAIClient {
    /// Create a new OpenAI client.
    pub fn new(config: OpenAIConfig) -> Result<Self, ProviderError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| ProviderError::Connection(e.to_string()))?;
        
        Ok(Self { config, client })
    }

    /// Get API endpoint.
    fn api_endpoint(&self) -> String {
        self.config.base_url
            .as_ref()
            .map(|u| format!("{}/chat/completions", u.trim_end_matches('/')))
            .unwrap_or_else(|| "https://api.openai.com/v1/chat/completions".to_string())
    }

    /// Convert hcode messages to OpenAI format.
    fn convert_messages(&self, messages: &[Message]) -> Vec<OpenAIMessage> {
        messages
            .iter()
            .filter_map(|msg| {
                match msg {
                    Message::User(user) => {
                        let content = user.message.content.iter()
                            .filter_map(|block| {
                                match block {
                                    ContentBlock::Text { text } => Some(text.clone()),
                                    _ => None,
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        
                        Some(OpenAIMessage {
                            role: "user".to_string(),
                            content: Some(content),
                            name: None,
                            tool_calls: None,
                        })
                    }
                    Message::Assistant(assistant) => {
                        let content = assistant.message.content.iter()
                            .filter_map(|block| {
                                match block {
                                    ContentBlock::Text { text } => Some(text.clone()),
                                    _ => None,
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        
                        Some(OpenAIMessage {
                            role: "assistant".to_string(),
                            content: Some(content),
                            name: None,
                            tool_calls: None,
                        })
                    }
                    _ => None,
                }
            })
            .collect()
    }
}

#[async_trait]
impl Provider for OpenAIClient {
    async fn complete(&self, messages: Vec<Message>, tools: Vec<hcode_types::ToolDefinition>) -> Result<Message, ProviderError> {
        let openai_messages = self.convert_messages(&messages);
        
        let openai_tools: Vec<ToolDefinition> = tools
            .iter()
            .map(|t| ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: t.name.clone(),
                    description: t.description.clone(),
                    parameters: t.parameters.clone(),
                },
            })
            .collect();
        
        let request = ChatCompletionRequest {
            model: self.config.model.clone(),
            messages: openai_messages,
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            tools: if openai_tools.is_empty() { None } else { Some(openai_tools) },
            stream: None,
        };
        
        let response = self.client
            .post(self.api_endpoint())
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| ProviderError::Connection(e.to_string()))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::Api(format!("HTTP {}: {}", status, body)));
        }
        
        let completion: ChatCompletionResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::Parse(e.to_string()))?;
        
        // Convert response to Message
        let choice = completion.choices.first()
            .ok_or_else(|| ProviderError::Parse("No choices in response".to_string()))?;
        
        let content: Vec<ContentBlock> = choice.message.content
            .as_ref()
            .map(|c| vec![ContentBlock::Text { text: c.clone() }])
            .unwrap_or_default();
        
        // Build assistant message
        let assistant = hcode_types::AssistantMessage {
            uuid: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            message: hcode_types::AssistantMessageContent {
                id: completion.id,
                role: hcode_types::Role::Assistant,
                model: completion.model,
                content,
                stop_reason: choice.finish_reason.clone(),
                stop_sequence: None,
                usage: completion.usage.map(|u| hcode_types::Usage {
                    input_tokens: u.prompt_tokens,
                    output_tokens: u.completion_tokens,
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: 0,
                }),
            },
            is_api_error_message: None,
            api_error: None,
            request_id: None,
        };
        
        Ok(Message::Assistant(assistant))
    }

    fn name(&self) -> &str {
        "openai"
    }
}