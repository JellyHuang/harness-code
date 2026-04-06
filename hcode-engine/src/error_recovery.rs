//! Error recovery patterns for query execution.
//!
//! Handles recovery from various API errors like
//! max_output_tokens, prompt_too_long, rate limits, model fallback.

use crate::compact::reactive::{reactive_compact, ReactiveTrigger};
use crate::compact::CompactionConfig;
use crate::state::{ContinueReason, LoopState};
use hcode_types::Message;

/// Maximum retries for max_output_tokens recovery
pub const MAX_OUTPUT_TOKENS_RECOVERY_LIMIT: u32 = 3;

/// Escalated max tokens (64k)
pub const ESCALATED_MAX_TOKENS: u32 = 64000;

/// Error recovery result
#[derive(Debug, Clone)]
pub enum RecoveryAction {
    /// Continue with recovery
    Continue {
        reason: ContinueReason,
        messages: Vec<Message>,
    },

    /// Retry with fallback model
    RetryWithFallback {
        fallback_model: String,
        original_model: String,
    },

    /// Abort the query
    Abort { error: String },
}

/// Error type classification
#[derive(Debug, Clone, PartialEq)]
pub enum ApiErrorType {
    /// Prompt too long (413 error)
    PromptTooLong {
        current_tokens: u64,
        max_tokens: u64,
    },
    /// Max output tokens reached
    MaxOutputTokens,
    /// Rate limit hit
    RateLimit { retry_after_ms: Option<u64> },
    /// Model overloaded (triggers fallback)
    ModelOverloaded,
    /// Authentication error
    AuthenticationError,
    /// Generic API error
    Generic { message: String },
}

/// Check if an error message indicates a recoverable error
pub fn classify_api_error(message: &str) -> ApiErrorType {
    let lower = message.to_lowercase();

    if lower.contains("prompt is too long") || lower.contains("prompt_too_long") {
        // Try to extract token counts
        ApiErrorType::PromptTooLong {
            current_tokens: 0,
            max_tokens: 0,
        }
    } else if lower.contains("max_output_tokens") || lower.contains("max output tokens") {
        ApiErrorType::MaxOutputTokens
    } else if lower.contains("rate limit") || lower.contains("rate_limit") {
        ApiErrorType::RateLimit {
            retry_after_ms: extract_retry_after(message),
        }
    } else if lower.contains("overloaded") || lower.contains("capacity") {
        ApiErrorType::ModelOverloaded
    } else if lower.contains("authentication") || lower.contains("invalid api key") {
        ApiErrorType::AuthenticationError
    } else {
        ApiErrorType::Generic {
            message: message.to_string(),
        }
    }
}

/// Extract retry-after value from error message
fn extract_retry_after(message: &str) -> Option<u64> {
    // Try to find retry-after pattern
    // Common patterns: "retry after 30s", "wait 5000ms"
    use regex::Regex;

    // Try seconds pattern
    if let Ok(re) = Regex::new(r"retry.*?(\d+)\s*s") {
        if let Some(caps) = re.captures(message) {
            if let Some(m) = caps.get(1) {
                if let Ok(secs) = m.as_str().parse::<u64>() {
                    return Some(secs * 1000);
                }
            }
        }
    }

    // Try milliseconds pattern
    if let Ok(re) = Regex::new(r"retry.*?(\d+)\s*ms") {
        if let Some(caps) = re.captures(message) {
            if let Some(m) = caps.get(1) {
                if let Ok(ms) = m.as_str().parse::<u64>() {
                    return Some(ms);
                }
            }
        }
    }

    None
}

/// Handle max_output_tokens recovery
///
/// Strategy:
/// 1. First, escalate to 64k tokens if using default
/// 2. If still hit limit, inject recovery message and retry
/// 3. After N retries, abort
pub async fn handle_max_output_tokens_recovery(
    state: &mut LoopState,
    current_override: Option<u32>,
    max_retries: u32,
) -> Option<MaxOutputTokensRecovery> {
    let current_attempt = state.max_output_tokens_recovery_count;

    // Check if we can escalate to 64k
    if current_override.is_none() {
        // First escalation: use 64k
        state.max_output_tokens_recovery_count += 1;
        return Some(MaxOutputTokensRecovery::Escalate {
            new_limit: ESCALATED_MAX_TOKENS,
        });
    }

    // Check if we have retries left
    if current_attempt < max_retries {
        state.max_output_tokens_recovery_count += 1;
        Some(MaxOutputTokensRecovery::RetryWithMessage {
            attempt: state.max_output_tokens_recovery_count,
        })
    } else {
        None
    }
}

/// Max output tokens recovery action
#[derive(Debug, Clone)]
pub enum MaxOutputTokensRecovery {
    /// Escalate to higher token limit
    Escalate { new_limit: u32 },
    /// Retry with recovery message
    RetryWithMessage { attempt: u32 },
}

/// Handle prompt_too_long recovery
///
/// Strategy:
/// 1. Check if reactive compact was already attempted
/// 2. If not, attempt reactive compact
/// 3. If compact succeeds, return with compacted messages
/// 4. If compact fails or already attempted, abort
pub async fn handle_prompt_too_long_recovery(
    state: &mut LoopState,
    current_tokens: u64,
    max_tokens: u64,
    compact_config: &CompactionConfig,
) -> Option<RecoveryAction> {
    // Don't try reactive compact twice
    if state.has_attempted_reactive_compact {
        return Some(RecoveryAction::Abort {
            error: "Prompt too long and reactive compaction already attempted".to_string(),
        });
    }

    state.has_attempted_reactive_compact = true;

    // Perform reactive compact
    let trigger = ReactiveTrigger::PromptTooLong {
        current_tokens,
        max_tokens,
    };

    let result = reactive_compact(state.messages.clone(), trigger, compact_config).await;

    match result {
        Ok(compaction_result) if compaction_result.compacted => Some(RecoveryAction::Continue {
            reason: ContinueReason::ReactiveCompactRetry,
            messages: compaction_result.messages,
        }),
        Ok(_) => {
            // Compaction ran but didn't help
            Some(RecoveryAction::Abort {
                error: "Prompt too long and reactive compaction insufficient".to_string(),
            })
        }
        Err(e) => Some(RecoveryAction::Abort {
            error: format!("Prompt too long and compaction failed: {}", e),
        }),
    }
}

/// Handle rate limit recovery
pub async fn handle_rate_limit_recovery(retry_after_ms: Option<u64>) -> RecoveryAction {
    // Wait if we have a retry-after value
    if let Some(delay_ms) = retry_after_ms {
        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
    }

    RecoveryAction::Continue {
        reason: ContinueReason::NextTurn,
        messages: vec![],
    }
}

/// Handle model fallback trigger
///
/// When a model is overloaded, switch to fallback model
pub fn handle_fallback_trigger(
    original_model: &str,
    fallback_model: Option<&str>,
) -> Option<RecoveryAction> {
    fallback_model.map(|model| RecoveryAction::RetryWithFallback {
        fallback_model: model.to_string(),
        original_model: original_model.to_string(),
    })
}

/// Check if an error message is a prompt-too-long error
pub fn is_prompt_too_long_error(message: &str) -> bool {
    let lower = message.to_lowercase();
    lower.contains("prompt is too long")
        || lower.contains("prompt_too_long")
        || lower.contains("context length exceeded")
        || lower.contains("maximum context length")
}

/// Check if an error message is a max-output-tokens error
pub fn is_max_output_tokens_error(message: &str) -> bool {
    let lower = message.to_lowercase();
    lower.contains("max_output_tokens") || lower.contains("max output tokens")
}

/// Check if an error indicates model overload (triggers fallback)
pub fn is_model_overload_error(message: &str) -> bool {
    let lower = message.to_lowercase();
    lower.contains("overloaded")
        || lower.contains("capacity")
        || lower.contains("temporarily unavailable")
}

/// Create recovery message for max_output_tokens
pub fn create_max_tokens_recovery_message() -> String {
    "Output token limit hit. Resume directly — no apology, no recap of what you were doing. \
     Pick up mid-thought if that is where the cut happened. Break remaining work into smaller pieces."
        .to_string()
}

/// Fallback model configuration
#[derive(Debug, Clone)]
pub struct FallbackConfig {
    /// Primary model
    pub primary_model: String,
    /// Fallback model (if any)
    pub fallback_model: Option<String>,
    /// Whether thinking signatures should be stripped on fallback
    pub strip_thinking_signatures: bool,
}

impl FallbackConfig {
    pub fn new(primary_model: String, fallback_model: Option<String>) -> Self {
        Self {
            primary_model,
            fallback_model,
            strip_thinking_signatures: true,
        }
    }

    /// Check if fallback is available
    pub fn has_fallback(&self) -> bool {
        self.fallback_model.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_prompt_too_long() {
        let error = "Error: prompt is too long: 200000 tokens > 190000 maximum";
        let classified = classify_api_error(error);
        assert!(matches!(classified, ApiErrorType::PromptTooLong { .. }));
    }

    #[test]
    fn test_classify_max_output_tokens() {
        let error = "Error: max_output_tokens limit exceeded";
        let classified = classify_api_error(error);
        assert!(matches!(classified, ApiErrorType::MaxOutputTokens));
    }

    #[test]
    fn test_classify_rate_limit() {
        let error = "Error: rate limit exceeded, retry after 30s";
        let classified = classify_api_error(error);
        assert!(matches!(classified, ApiErrorType::RateLimit { .. }));
    }

    #[test]
    fn test_is_prompt_too_long() {
        assert!(is_prompt_too_long_error("prompt is too long"));
        assert!(is_prompt_too_long_error("PROMPT_TOO_LONG"));
        assert!(is_prompt_too_long_error("context length exceeded"));
        assert!(!is_prompt_too_long_error("some other error"));
    }

    #[test]
    fn test_is_max_output_tokens() {
        assert!(is_max_output_tokens_error("max_output_tokens exceeded"));
        assert!(is_max_output_tokens_error("Max output tokens limit"));
        assert!(!is_max_output_tokens_error("other error"));
    }

    #[test]
    fn test_is_model_overload() {
        assert!(is_model_overload_error("model overloaded"));
        assert!(is_model_overload_error("insufficient capacity"));
        assert!(!is_model_overload_error("other error"));
    }

    #[tokio::test]
    async fn test_max_output_tokens_recovery() {
        let mut state = LoopState::new(vec![]);

        // First call should escalate
        let result = handle_max_output_tokens_recovery(&mut state, None, 3).await;
        assert!(matches!(
            result,
            Some(MaxOutputTokensRecovery::Escalate { new_limit: 64000 })
        ));

        // With override, should retry
        let result = handle_max_output_tokens_recovery(&mut state, Some(8000), 3).await;
        assert!(matches!(
            result,
            Some(MaxOutputTokensRecovery::RetryWithMessage { .. })
        ));

        // Exhaust retries
        state.max_output_tokens_recovery_count = 3;
        let result = handle_max_output_tokens_recovery(&mut state, Some(8000), 3).await;
        assert!(result.is_none());
    }

    #[test]
    fn test_fallback_config() {
        let config = FallbackConfig::new(
            "claude-sonnet".to_string(),
            Some("claude-haiku".to_string()),
        );
        assert!(config.has_fallback());
        assert_eq!(config.primary_model, "claude-sonnet");
        assert_eq!(config.fallback_model, Some("claude-haiku".to_string()));
    }
}
