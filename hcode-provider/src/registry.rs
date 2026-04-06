//! Provider registry.

use crate::{AnthropicClient, Provider};
use hcode_config::Config;
use std::collections::HashMap;
use std::sync::Arc;

/// Registry for managing multiple providers.
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn Provider>>,
    default: Option<String>,
}

impl ProviderRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            default: None,
        }
    }

    /// Create a registry from configuration.
    ///
    /// This will create provider instances based on the configuration.
    /// Only enabled providers will be instantiated.
    pub fn from_config(config: &Config) -> Self {
        let mut registry = Self::new();

        // Get the list of providers to create
        let provider_ids: Vec<&String> = config.provider.keys().collect();

        for provider_id in provider_ids {
            // Skip disabled providers
            if !config.is_provider_enabled(provider_id) {
                continue;
            }

            // Get provider config
            let provider_config = match config.provider.get(provider_id) {
                Some(p) => p,
                None => continue,
            };

            // Get API key
            let api_key = match &provider_config.options.api_key {
                Some(key) if !key.is_empty() => Some(key.clone()),
                _ => {
                    // Try to get from environment variable based on provider
                    // Support both ANTHROPIC_API_KEY and ANTHROPIC_AUTH_TOKEN for Anthropic
                    match provider_id.as_str() {
                        "anthropic" => std::env::var("ANTHROPIC_API_KEY")
                            .or_else(|_| std::env::var("ANTHROPIC_AUTH_TOKEN"))
                            .ok(),
                        "openai" => std::env::var("OPENAI_API_KEY").ok(),
                        "openrouter" => std::env::var("OPENROUTER_API_KEY").ok(),
                        "minimax" => std::env::var("MINIMAX_API_KEY").ok(),
                        "dashscope" => std::env::var("DASHSCOPE_API_KEY").ok(),
                        _ => None,
                    }
                }
            };

            let api_key = match api_key {
                Some(key) => key,
                None => {
                    eprintln!(
                        "Warning: No API key for provider '{}', skipping",
                        provider_id
                    );
                    continue;
                }
            };

            // Get model
            let model = config
                .model
                .as_ref()
                .and_then(|m| {
                    let (provider, model) = Config::parse_model_string(m);
                    if provider == Some(provider_id.as_str()) {
                        Some(model.to_string())
                    } else {
                        None
                    }
                })
                .or_else(|| {
                    provider_config
                        .models
                        .as_ref()
                        .and_then(|m| m.keys().next().cloned())
                })
                .unwrap_or_else(|| "claude-sonnet-4-20250514".to_string());

            // Create provider instance based on type
            // For unknown providers, treat them as Anthropic-compatible APIs
            let base_url = match provider_id.as_str() {
                "anthropic" => provider_config
                    .options
                    .base_url
                    .clone()
                    .unwrap_or_else(|| "https://api.anthropic.com".to_string()),
                "openrouter" => provider_config
                    .options
                    .base_url
                    .clone()
                    .unwrap_or_else(|| "https://openrouter.ai/api/v1".to_string()),
                "minimax" => provider_config
                    .options
                    .base_url
                    .clone()
                    .unwrap_or_else(|| "https://api.minimaxi.com/anthropic".to_string()),
                _ => {
                    // For custom providers, require base_url
                    match &provider_config.options.base_url {
                        Some(url) => url.clone(),
                        None => {
                            eprintln!(
                                "Warning: Custom provider '{}' requires 'baseURL' option, skipping",
                                provider_id
                            );
                            continue;
                        }
                    }
                }
            };

            let client = AnthropicClient::with_name(provider_id, &api_key, &model, &base_url);
            registry.register(provider_id, Arc::new(client));
        }

        // Set default provider
        if let Some(model) = &config.model {
            let (provider, _) = Config::parse_model_string(model);
            if let Some(provider) = provider {
                registry.set_default(provider);
            }
        } else if !registry.providers.is_empty() {
            // Use first provider as default
            let first = registry.providers.keys().next().unwrap().clone();
            registry.default = Some(first);
        }

        registry
    }

    /// Register a provider.
    pub fn register(&mut self, name: impl Into<String>, provider: Arc<dyn Provider>) {
        let name = name.into();
        if self.providers.is_empty() {
            self.default = Some(name.clone());
        }
        self.providers.insert(name, provider);
    }

    /// Get a provider by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Provider>> {
        self.providers.get(name).cloned()
    }

    /// Get the default provider.
    pub fn get_default(&self) -> Option<Arc<dyn Provider>> {
        self.default.as_ref().and_then(|n| self.get(n))
    }

    /// Set the default provider.
    pub fn set_default(&mut self, name: &str) {
        if self.providers.contains_key(name) {
            self.default = Some(name.to_string());
        }
    }

    /// List all provider names.
    pub fn list(&self) -> Vec<&str> {
        self.providers.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_registry() {
        let registry = ProviderRegistry::new();
        assert!(registry.get("test").is_none());
        assert!(registry.get_default().is_none());
    }

    #[test]
    fn test_registry_from_empty_config() {
        let config = Config::default();
        let registry = ProviderRegistry::from_config(&config);
        assert!(registry.list().is_empty());
    }
}
