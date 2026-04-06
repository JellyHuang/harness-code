//! Query state machine for managing query lifecycle.
//!
//! Implements enum-based state transitions matching TypeScript's query loop.

use hcode_types::{Message, Usage};
use tokio_util::sync::CancellationToken;

/// Query engine state machine
#[derive(Debug)]
pub enum QueryState {
    /// Initial state before query starts
    Initial,

    /// Streaming from LLM API
    StreamingApi { turn: u32, usage: Usage },

    /// Executing tools
    ToolExecution {
        pending_blocks: Vec<ToolUseBlock>,
        concurrent: bool,
    },

    /// Running compaction
    Compaction { trigger: CompactionTrigger },

    /// Running stop hooks
    StopHooks { assistant_messages: Vec<Message> },

    /// Terminal state
    Terminal { reason: TerminalReason },
}

impl Default for QueryState {
    fn default() -> Self {
        Self::Initial
    }
}

/// Tool use block for execution
#[derive(Debug, Clone)]
pub struct ToolUseBlock {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
}

/// Compaction trigger reason
#[derive(Debug, Clone)]
pub enum CompactionTrigger {
    /// Automatic based on token threshold
    Auto { token_threshold: u64 },
    /// Manual compaction request
    Manual,
    /// Reactive (triggered by API error)
    Reactive { error: String },
}

/// Terminal reason for query completion
#[derive(Debug, Clone)]
pub enum TerminalReason {
    /// Normal completion
    Completed,
    /// Max turns reached
    MaxTurns { turn_count: u32, max_turns: u32 },
    /// Budget exceeded
    BudgetExceeded { cost_usd: f64, max_budget_usd: f64 },
    /// Cancelled by user
    Cancelled,
    /// Error during execution
    Error { message: String },
    /// Stop hook prevented continuation
    StopHookPrevented,
    /// Hook stopped execution
    HookStopped,
}

/// Reason for continuing to next iteration
#[derive(Debug, Clone)]
pub enum ContinueReason {
    /// Continue to next turn
    NextTurn,
    /// Max output tokens recovery
    MaxOutputTokensRecovery { attempt: u32 },
    /// Reactive compact retry
    ReactiveCompactRetry,
    /// Stop hook blocking
    StopHookBlocking,
    /// Token budget continuation
    TokenBudgetContinuation,
    /// Collapse drain retry
    CollapseDrainRetry { committed: usize },
}

/// Mutable state carried between loop iterations
#[derive(Debug)]
pub struct LoopState {
    /// Messages in conversation
    pub messages: Vec<Message>,

    /// Token usage tracking
    pub total_usage: Usage,

    /// Turn counter
    pub turn_count: u32,

    /// Max output tokens recovery attempts
    pub max_output_tokens_recovery_count: u32,

    /// Whether reactive compact was attempted
    pub has_attempted_reactive_compact: bool,

    /// Stop hook active flag
    pub stop_hook_active: bool,

    /// Continue reason from previous iteration
    pub transition: Option<ContinueReason>,

    /// Cancellation token
    pub cancel_token: CancellationToken,
}

impl LoopState {
    pub fn new(messages: Vec<Message>) -> Self {
        Self {
            messages,
            total_usage: Usage::default(),
            turn_count: 1,
            max_output_tokens_recovery_count: 0,
            has_attempted_reactive_compact: false,
            stop_hook_active: false,
            transition: None,
            cancel_token: CancellationToken::new(),
        }
    }

    /// Check if cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancel_token.is_cancelled()
    }

    /// Cancel the query
    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }
}

/// Auto-compact tracking state
#[derive(Debug, Clone)]
pub struct AutoCompactTracking {
    /// Whether compaction has occurred
    pub compacted: bool,

    /// Unique turn ID
    pub turn_id: String,

    /// Turn counter since last compact
    pub turn_counter: u32,

    /// Consecutive failure count
    pub consecutive_failures: u32,
}

impl Default for AutoCompactTracking {
    fn default() -> Self {
        Self {
            compacted: false,
            turn_id: uuid::Uuid::new_v4().to_string(),
            turn_counter: 0,
            consecutive_failures: 0,
        }
    }
}

/// Query chain tracking for analytics
#[derive(Debug, Clone)]
pub struct QueryChainTracking {
    pub chain_id: String,
    pub depth: u32,
}

impl Default for QueryChainTracking {
    fn default() -> Self {
        Self {
            chain_id: uuid::Uuid::new_v4().to_string(),
            depth: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_transitions() {
        let mut state = QueryState::Initial;

        // Initial -> Streaming
        state = QueryState::StreamingApi {
            turn: 1,
            usage: Usage::default(),
        };

        // Streaming -> ToolExecution
        state = QueryState::ToolExecution {
            pending_blocks: vec![],
            concurrent: true,
        };

        // ToolExecution -> Terminal
        state = QueryState::Terminal {
            reason: TerminalReason::Completed,
        };

        match state {
            QueryState::Terminal { reason } => {
                assert!(matches!(reason, TerminalReason::Completed));
            }
            _ => panic!("Expected terminal state"),
        }
    }

    #[test]
    fn test_loop_state() {
        let state = LoopState::new(vec![]);
        assert_eq!(state.turn_count, 1);
        assert!(!state.is_cancelled());

        state.cancel();
        assert!(state.is_cancelled());
    }
}
