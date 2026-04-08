//! OpenAI client implementation.

use super::types::*;
use crate::{Provider, ProviderError};
use async_trait::async_trait;
use hcode_types::{ContentBlock, Message};
use reqwest::Client;

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
    fn name(&self) -> &str {
        "openai"
    }

    fn model(&self) -> &str {
        &self.config.model
    }

    async fn stream(
        &self,
        _messages: Vec<Message>,
        _tools: Vec<hcode_types::ToolDefinition>,
        _system_prompt: Option<String>,
    ) -> Result<std::pin::Pin<Box<dyn futures::Stream<Item = hcode_protocol::StreamEvent> + Send>>, ProviderError> {
        // TODO: Implement streaming
        Err(ProviderError::Stream("Streaming not yet implemented".to_string()))
    }

    async fn complete(&self, messages: Vec<Message>, tools: Vec<hcode_types::ToolDefinition>) -> Result<crate::CompletionResponse, ProviderError> {
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
        
        // Convert response to CompletionResponse
        let choice = completion.choices.first()
            .ok_or_else(|| ProviderError::Parse("No choices in response".to_string()))?;
        
        let content = choice.message.content.clone().unwrap_or_default();
        
        // Extract tool calls if any
        let tool_calls: Vec<crate::ToolCall> = choice.message.tool_calls
            .as_ref()
            .map(|tc| tc.iter().map(|t| crate::ToolCall {
                id: t.id.clone(),
                name: t.function.name.clone(),
                input: serde_json::from_str(&t.function.arguments).unwrap_or(serde_json::Value::Null),
            }).collect())
            .unwrap_or_default();
        
        let usage = crate::StreamUsage {
            input_tokens: completion.usage.as_ref().map(|u| u.prompt_tokens as u32).unwrap_or(0),
            output_tokens: completion.usage.as_ref().map(|u| u.completion_tokens as u32).unwrap_or(0),
        };
        
        Ok(crate::CompletionResponse {
            content,
            tool_calls,
            usage,
            stop_reason: choice.finish_reason.clone().unwrap_or_default(),
        })
    }
}