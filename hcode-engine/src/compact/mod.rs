//! Compaction strategies for context management.
//!
//! Implements microcompact, autocompact, and reactive compact
//! to manage context window and token budgets.

pub mod auto;
pub mod micro;
pub mod reactive;

pub use auto::*;
pub use micro::*;
pub use reactive::*;

use hcode_types::Message;

/// Compaction result
#[derive(Debug, Clone)]
pub struct CompactionResult {
    /// Messages after compaction
    pub messages: Vec<Message>,

    /// Tokens saved by compaction
    pub tokens_saved: u64,

    /// Whether compaction was performed
    pub compacted: bool,
}

/// Compaction config
#[derive(Debug, Clone)]
pub struct CompactionConfig {
    /// Auto-compact threshold (tokens)
    pub auto_compact_threshold: u64,

    /// Buffer tokens to reserve
    pub buffer_tokens: u64,

    /// Maximum consecutive failures
    pub max_consecutive_failures: u32,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            auto_compact_threshold: 150_000,
            buffer_tokens: 13_000,
            max_consecutive_failures: 3,
        }
    }
}
