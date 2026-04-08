//! Anthropic API client.

use crate::{CompletionResponse, Provider, ProviderError, StreamUsage, ToolCall};
use async_trait::async_trait;
use futures::Stream;
use hcode_protocol::StreamEvent;
use hcode_types::{Message, ToolDefinition};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use std::pin::Pin;

/// Anthropic API client.
pub struct AnthropicClient {
    name: String,
    api_key: String,
    model: String,
    base_url: String,
    http_client: reqwest::Client,
}

use crate::anthropic::stream::AnthropicStream;
use crate::anthropic::types::*;

impl AnthropicClient {
    /// Create a new Anthropic client.
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            name: "anthropic".to_string(),
            api_key: api_key.into(),
            model: model.into(),
            base_url: "https://api.anthropic.com".to_string(),
            http_client: reqwest::Client::new(),
        }
    }

    /// Create with custom base URL (for proxies like OpenRouter).
    pub fn with_base_url(
        api_key: impl Into<String>,
        model: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Self {
        Self {
            name: "anthropic".to_string(),
            api_key: api_key.into(),
            model: model.into(),
            base_url: base_url.into(),
            http_client: reqwest::Client::new(),
        }
    }

    /// Create with custom name, base URL, and model.
    pub fn with_name(
        name: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            api_key: api_key.into(),
            model: model.into(),
            base_url: base_url.into(),
            http_client: reqwest::Client::new(),
        }
    }

    /// Build request headers.
    fn build_headers(&self) -> Result<HeaderMap, ProviderError> {
        let mut headers = HeaderMap::new();
        // Use Bearer token for auth (matches opencode/cc-haha pattern)
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.api_key))
                .map_err(|e| ProviderError::Auth(e.to_string()))?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        // Add User-Agent to identify as a coding agent client
        headers.insert(
            "User-Agent",
            HeaderValue::from_static("hcode/0.1.0 (AI Coding Agent)"),
        );
        Ok(headers)
    }

    /// Get the messages endpoint URL.
    fn messages_url(&self) -> String {
        // Handle base URLs that already have /v1 suffix
        let base = self.base_url.trim_end_matches('/');
        if base.ends_with("/v1") {
            format!("{}/messages", base)
        } else {
            format!("{}/v1/messages", base)
        }
    }

    /// Build request body.
    fn build_request(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolDefinition>,
        system_prompt: Option<String>,
        stream: bool,
    ) -> Result<MessagesRequest, ProviderError> {
        // Use provided system prompt or default
        let system = system_prompt.or_else(|| {
            // Default minimal system prompt
            Some("You are a helpful AI assistant.".to_string())
        });

        let anthropic_messages: Vec<AnthropicMessage> = messages
            .into_iter()
            .filter_map(|msg| {
                // Filter messages - skip non-user/assistant
                let should_include = matches!(
                    &msg,
                    hcode_types::Message::User(_) | hcode_types::Message::Assistant(_)
                );

                if should_include {
                    Some(AnthropicMessage::from(msg))
                } else {
                    None
                }
            })
            .collect();

        let anthropic_tools = if tools.is_empty() {
            None
        } else {
            Some(
                tools
                    .into_iter()
                    .map(AnthropicToolDefinition::from)
                    .collect(),
            )
        };

        Ok(MessagesRequest {
            model: self.model.clone(),
            messages: anthropic_messages,
            tools: anthropic_tools,
            max_tokens: 4096,
            system,
            thinking: None, // Could be configurable
            stream,
        })
    }

    /// Make a non-streaming request.
    async fn complete_internal(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolDefinition>,
    ) -> Result<CompletionResponse, ProviderError> {
        let request = self.build_request(messages, tools, None, false)?;
        let headers = self.build_headers()?;

        let response = self
            .http_client
            .post(self.messages_url())
            .headers(headers)
            .json(&request)
            .send()
            .await
            .map_err(ProviderError::Http)?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ProviderError::Api(format!(
                "HTTP {}: {}",
                status.as_u16(),
                error_text
            )));
        }

        let response_body: serde_json::Value =
            response.json().await.map_err(ProviderError::Http)?;

        // Parse the response
        let content = response_body["content"]
            .as_array()
            .and_then(|arr| arr.iter().filter_map(|c| c["text"].as_str()).next())
            .unwrap_or_default()
            .to_string();

        let usage = StreamUsage {
            input_tokens: response_body["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32,
            output_tokens: response_body["usage"]["output_tokens"]
                .as_u64()
                .unwrap_or(0) as u32,
        };

        let stop_reason = response_body["stop_reason"]
            .as_str()
            .unwrap_or("end_turn")
            .to_string();

        // Parse tool calls
        let mut tool_calls = Vec::new();
        if let Some(content_array) = response_body["content"].as_array() {
            for block in content_array {
                if block["type"] == "tool_use" {
                    tool_calls.push(ToolCall {
                        id: block["id"].as_str().unwrap_or_default().to_string(),
                        name: block["name"].as_str().unwrap_or_default().to_string(),
                        input: block["input"].clone(),
                    });
                }
            }
        }

        Ok(CompletionResponse {
            content,
            tool_calls,
            usage,
            stop_reason,
        })
    }

    /// Make a streaming request.
    async fn stream_internal(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolDefinition>,
        system_prompt: Option<String>,
    ) -> Result<Pin<Box<dyn Stream<Item = StreamEvent> + Send>>, ProviderError> {
        let request = self.build_request(messages, tools, system_prompt, true)?;
        let headers = self.build_headers()?;

        let response = self
            .http_client
            .post(self.messages_url())
            .headers(headers)
            .json(&request)
            .send()
            .await
            .map_err(ProviderError::Http)?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ProviderError::Api(format!(
                "HTTP {}: {}",
                status.as_u16(),
                error_text
            )));
        }

        let stream = AnthropicStream::new(response);
        Ok(Box::pin(stream))
    }
}

#[async_trait]
impl Provider for AnthropicClient {
    fn name(&self) -> &str {
        &self.name
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn stream(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolDefinition>,
        system_prompt: Option<String>,
    ) -> Result<Pin<Box<dyn Stream<Item = StreamEvent> + Send>>, ProviderError> {
        self.stream_internal(messages, tools, system_prompt).await
    }

    async fn complete(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolDefinition>,
    ) -> Result<CompletionResponse, ProviderError> {
        self.complete_internal(messages, tools).await
    }
}
