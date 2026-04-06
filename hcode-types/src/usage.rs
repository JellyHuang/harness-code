//! Usage tracking types.

use serde::{Deserialize, Serialize};

/// Token usage for an LLM request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Number of input tokens.
    pub input_tokens: u32,
    /// Number of output tokens.
    pub output_tokens: u32,
    /// Number of cache read tokens (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_tokens: Option<u32>,
    /// Number of cache write tokens (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_write_tokens: Option<u32>,
}

impl TokenUsage {
    /// Total tokens used.
    pub fn total(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }
}

/// Duration tracking.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Duration {
    /// Duration in milliseconds.
    pub ms: u64,
}

impl Duration {
    /// Create a new duration.
    pub fn from_millis(ms: u64) -> Self {
        Self { ms }
    }

    /// Create from seconds.
    pub fn from_secs(secs: u64) -> Self {
        Self { ms: secs * 1000 }
    }
}

impl From<std::time::Duration> for Duration {
    fn from(d: std::time::Duration) -> Self {
        Self {
            ms: d.as_millis() as u64,
        }
    }
}

impl From<Duration> for std::time::Duration {
    fn from(d: Duration) -> Self {
        std::time::Duration::from_millis(d.ms)
    }
}
