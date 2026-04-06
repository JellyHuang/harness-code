//! Micro-compact implementation.
//!
//! Performs small-scale compaction within a turn.

use super::{CompactionConfig, CompactionResult};
use hcode_types::Message;

/// Perform micro-compaction
pub async fn micro_compact(
    messages: Vec<Message>,
    _config: &CompactionConfig,
) -> Result<CompactionResult, CompactError> {
    // Micro-compact focuses on recent turn optimization
    // Simplified implementation

    Ok(CompactionResult {
        messages,
        tokens_saved: 0,
        compacted: false,
    })
}

/// Compact error (re-exported from auto)
pub use super::auto::CompactError;
