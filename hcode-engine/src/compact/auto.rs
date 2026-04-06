//! Auto-compact implementation.
//!
//! Automatically compacts conversation when token threshold is exceeded.

use super::{CompactionConfig, CompactionResult};
use hcode_types::Message;

/// Check if auto-compact should be triggered
pub fn should_auto_compact(
    _messages: &[Message],
    current_tokens: u64,
    config: &CompactionConfig,
    query_source: Option<&str>,
) -> bool {
    // Skip for certain query sources
    if let Some(source) = query_source {
        if source == "session_memory" || source == "compact" {
            return false;
        }
    }

    // Check threshold
    current_tokens >= config.auto_compact_threshold
}

/// Perform auto-compaction
pub async fn auto_compact(
    messages: Vec<Message>,
    config: &CompactionConfig,
) -> Result<CompactionResult, CompactError> {
    // Calculate current token count (simplified)
    let current_tokens = estimate_tokens(&messages);

    // Check if compaction needed
    if current_tokens < config.auto_compact_threshold {
        return Ok(CompactionResult {
            messages,
            tokens_saved: 0,
            compacted: false,
        });
    }

    // Perform compaction (simplified - would call LLM for summarization)
    let compacted_messages = compact_messages(messages, config).await?;

    let new_tokens = estimate_tokens(&compacted_messages);
    let tokens_saved = current_tokens.saturating_sub(new_tokens);

    Ok(CompactionResult {
        messages: compacted_messages,
        tokens_saved,
        compacted: true,
    })
}

/// Compact messages by summarizing old content
async fn compact_messages(
    messages: Vec<Message>,
    _config: &CompactionConfig,
) -> Result<Vec<Message>, CompactError> {
    // Simplified implementation - in production would:
    // 1. Keep recent messages (tail)
    // 2. Summarize older messages using LLM
    // 3. Insert summary as a system message

    // For now, just return last N messages
    let keep_count = messages.len().saturating_sub(10);
    let compacted: Vec<Message> = messages.into_iter().skip(keep_count).collect();

    Ok(compacted)
}

/// Estimate token count for messages (simplified)
fn estimate_tokens(messages: &[Message]) -> u64 {
    // Simplified estimation - would use proper tokenizer
    let mut tokens = 0u64;

    for msg in messages {
        match msg {
            Message::User(m) => {
                for block in &m.message.content {
                    if let hcode_types::ContentBlock::Text { text } = block {
                        tokens += (text.len() / 4) as u64; // Rough estimate
                    }
                }
            }
            Message::Assistant(m) => {
                for block in &m.message.content {
                    if let hcode_types::ContentBlock::Text { text } = block {
                        tokens += (text.len() / 4) as u64;
                    }
                }
            }
            _ => {}
        }
    }

    tokens
}

/// Compact error
#[derive(Debug, thiserror::Error)]
pub enum CompactError {
    #[error("Compaction failed: {0}")]
    Failed(String),

    #[error("Token estimation failed: {0}")]
    TokenEstimation(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_auto_compact() {
        let config = CompactionConfig::default();
        let messages = vec![];

        // Below threshold
        assert!(!should_auto_compact(&messages, 1000, &config, None));

        // Above threshold
        assert!(should_auto_compact(&messages, 200_000, &config, None));
    }
}
