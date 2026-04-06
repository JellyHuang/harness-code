//! Error types for HCode.

use thiserror::Error;

/// The main error type for HCode.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),

    #[error("Tool error: {0}")]
    Tool(#[from] ToolError),

    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Errors from LLM providers.
#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("API error: {0}")]
    Api(String),

    #[error("Stream error: {0}")]
    Stream(String),

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Rate limited: {0}")]
    RateLimited(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Request timeout")]
    Timeout,
}

/// Errors from tool execution.
#[derive(Debug, Error)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),

    #[error("Permission denied: {0}")]
    Permission(String),

    #[error("Execution failed: {0}")]
    Execution(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Timeout")]
    Timeout,
}

/// Errors from protocol handling.
#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("XML parse error: {0}")]
    XmlParse(String),

    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    #[error("SSE parse error: {0}")]
    SseParse(String),
}

/// Errors from configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Config file not found: {0}")]
    NotFound(String),

    #[error("Invalid config: {0}")]
    Invalid(String),

    #[error("YAML parse error: {0}")]
    Yaml(String),
}

/// Result type alias for HCode.
pub type Result<T> = std::result::Result<T, AppError>;
