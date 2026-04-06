//! Configuration types compatible with OpenCode format.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// JSON schema reference.
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    /// Default model to use (format: provider/model or just model).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Small model for lightweight tasks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub small_model: Option<String>,

    /// Provider configurations (key is provider-id).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub provider: HashMap<String, ProviderConfig>,

    /// Agent configurations (key is agent name).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub agents: HashMap<String, AgentConfig>,

    /// List of enabled provider IDs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled_providers: Option<Vec<String>>,

    /// List of disabled provider IDs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled_providers: Option<Vec<String>>,

    /// Data directory for session storage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_dir: Option<String>,

    /// Debug mode.
    #[serde(default)]
    pub debug: bool,

    /// Plugin configurations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin: Option<Vec<String>>,
}

/// Provider configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Provider options (apiKey, baseURL, timeout, etc).
    #[serde(default)]
    pub options: ProviderOptions,

    /// Model-specific configurations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub models: Option<HashMap<String, ModelConfig>>,

    /// Whether this provider is disabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
}

/// Provider options.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderOptions {
    /// API key for authentication.
    #[serde(rename = "apiKey", skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    /// Base URL for API requests (for proxies).
    #[serde(rename = "baseURL", skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,

    /// Request timeout in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,

    /// Chunk timeout for streaming in milliseconds.
    #[serde(rename = "chunkTimeout", skip_serializing_if = "Option::is_none")]
    pub chunk_timeout: Option<u64>,

    /// Custom headers for requests.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub headers: HashMap<String, String>,
}

/// Model configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Display name for the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Model ID override (for custom inference profiles).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Token limits for the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<ModelLimit>,
}

/// Model token limits.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelLimit {
    /// Maximum context window size.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<u64>,

    /// Maximum output tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<u64>,
}

/// Agent configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Agent description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Model to use for this agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// System prompt for the agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,

    /// Maximum tokens for responses.
    #[serde(rename = "maxTokens", skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Tool permissions (tool name -> permission).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub tools: HashMap<String, ToolPermission>,
}

/// Tool permission setting.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolPermission {
    /// Simple boolean: true = enabled, false = disabled.
    Boolean(bool),
    /// Detailed configuration.
    Detailed(HashMap<String, String>),
}

// ============================================================================
// Legacy types for backward compatibility
// ============================================================================

/// Legacy agent definition (for migration).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub name: String,
    pub agent_type: String,
    pub model: Option<String>,
    #[serde(default)]
    pub tools: Vec<String>,
}

impl Config {
    /// Create a new empty config.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a provider is enabled.
    pub fn is_provider_enabled(&self, provider_id: &str) -> bool {
        // Check disabled first (takes precedence)
        if let Some(disabled) = &self.disabled_providers {
            if disabled.iter().any(|d| d == provider_id) {
                return false;
            }
        }

        // Check enabled list
        if let Some(enabled) = &self.enabled_providers {
            return enabled.iter().any(|e| e == provider_id);
        }

        // Check provider's disabled flag
        if let Some(provider) = self.provider.get(provider_id) {
            return provider.disabled.unwrap_or(false) == false;
        }

        // Default: not configured, so not enabled
        false
    }

    /// Get the effective API key for a provider.
    pub fn get_api_key(&self, provider_id: &str) -> Option<&str> {
        self.provider
            .get(provider_id)
            .and_then(|p| p.options.api_key.as_deref())
    }

    /// Get the effective base URL for a provider.
    pub fn get_base_url(&self, provider_id: &str) -> Option<&str> {
        self.provider
            .get(provider_id)
            .and_then(|p| p.options.base_url.as_deref())
    }

    /// Parse a model string like "anthropic/claude-sonnet-4" into (provider, model).
    pub fn parse_model_string(model: &str) -> (Option<&str>, &str) {
        match model.split_once('/') {
            Some((provider, model)) => (Some(provider), model),
            None => (None, model),
        }
    }
}
