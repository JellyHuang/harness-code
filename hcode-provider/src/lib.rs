//! HCode Provider - LLM provider abstraction.

pub mod anthropic;
pub mod openai;
pub mod provider;
pub mod registry;

// Explicitly re-export types to avoid ambiguous glob re-exports
pub use anthropic::{AnthropicClient, AnthropicStream};
pub use openai::{OpenAIClient, OpenAIConfig, OpenAIProvider};
pub use provider::{Provider, CompletionResponse, StreamUsage, ProviderError};
pub use registry::*;

// Re-export types for convenience
pub use hcode_types::{Message, ToolDefinition};

// Re-export ToolCall from provider module (canonical version)
pub use provider::ToolCall;
