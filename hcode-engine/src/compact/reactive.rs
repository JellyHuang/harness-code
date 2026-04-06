//! Reactive compact implementation.
//!
//! Compaction triggered by API errors (prompt_too_long, max_tokens).

use super::{CompactionConfig, CompactionResult};
use hcode_types::Message;

/// Reactive compaction trigger
#[derive(Debug, Clone)]
pub enum ReactiveTrigger {
    /// Prompt too long error
    PromptTooLong {
        current_tokens: u64,
        max_tokens: u64,
    },

    /// Max output tokens reached
    MaxOutputTokens { attempt: u32 },
}

/// Perform reactive compaction
pub async fn reactive_compact(
    messages: Vec<Message>,
    trigger: ReactiveTrigger,
    config: &CompactionConfig,
) -> Result<CompactionResult, CompactError> {
    // Reactive compact is more aggressive than auto-compact
    match trigger {
        ReactiveTrigger::PromptTooLong {
            current_tokens,
            max_tokens,
        } => {
            // Need to reduce by at least the excess
            let excess = current_tokens.saturating_sub(max_tokens);
            aggressive_compact(messages, excess, config).await
        }
        ReactiveTrigger::MaxOutputTokens { attempt: _ } => {
            // Less aggressive for max_output_tokens recovery
            moderate_compact(messages, config).await
        }
    }
}

/// Aggressive compaction for prompt_too_long
async fn aggressive_compact(
    messages: Vec<Message>,
    target_reduction: u64,
    _config: &CompactionConfig,
) -> Result<CompactionResult, CompactError> {
    // Remove older messages until we meet target
    let mut tokens_to_remove = target_reduction;
    let mut compacted = false;

    let result_messages: Vec<Message> = messages
        .into_iter()
        .rev()
        .filter(|msg| {
            if tokens_to_remove > 0 {
                // Estimate tokens in this message
                let msg_tokens = estimate_message_tokens(msg);
                if tokens_to_remove >= msg_tokens {
                    tokens_to_remove = tokens_to_remove.saturating_sub(msg_tokens);
                    compacted = true;
                    false // Remove this message
                } else {
                    true // Keep this message
                }
            } else {
                true
            }
        })
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    Ok(CompactionResult {
        messages: result_messages,
        tokens_saved: target_reduction.saturating_sub(tokens_to_remove),
        compacted,
    })
}

/// Moderate compaction for max_output_tokens recovery
async fn moderate_compact(
    messages: Vec<Message>,
    _config: &CompactionConfig,
) -> Result<CompactionResult, CompactError> {
    // Less aggressive - just trim older messages
    let keep_count = (messages.len() as f64 * 0.8) as usize;
    let compacted = messages.len() > keep_count;

    let result_messages: Vec<Message> = messages
        .into_iter()
        .rev()
        .take(keep_count)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    Ok(CompactionResult {
        messages: result_messages,
        tokens_saved: 0,
        compacted,
    })
}

/// Estimate tokens in a message
fn estimate_message_tokens(_msg: &Message) -> u64 {
    // Simplified - would use proper tokenizer
    100
}

/// Compact error (re-exported from auto)
pub use super::auto::CompactError;
