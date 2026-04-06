//! Stop hooks system for query lifecycle events.
//!
//! Implements hooks that run after each turn to evaluate
//! continuation conditions and potentially block further execution.

use hcode_types::Message;

/// Stop hook result
#[derive(Debug, Clone)]
pub struct StopHookResult {
    /// Whether to prevent continuation
    pub prevent_continuation: bool,

    /// Blocking errors to display
    pub blocking_errors: Vec<String>,

    /// Hook messages
    pub messages: Vec<String>,
}

impl Default for StopHookResult {
    fn default() -> Self {
        Self {
            prevent_continuation: false,
            blocking_errors: vec![],
            messages: vec![],
        }
    }
}

/// Stop hook trait
#[async_trait::async_trait]
pub trait StopHook: Send + Sync {
    /// Hook name
    fn name(&self) -> &str;

    /// Execute the hook
    async fn execute(
        &self,
        messages: &[Message],
        assistant_messages: &[Message],
        context: &StopHookContext,
    ) -> Result<StopHookResult, StopHookError>;
}

/// Stop hook context
#[derive(Debug, Clone)]
pub struct StopHookContext {
    /// Current turn
    pub turn: u32,

    /// Max turns allowed
    pub max_turns: Option<u32>,

    /// Current cost
    pub current_cost_usd: f64,

    /// Max budget
    pub max_budget_usd: Option<f64>,
}

/// Stop hook error
#[derive(Debug, thiserror::Error)]
pub enum StopHookError {
    #[error("Hook execution failed: {0}")]
    Execution(String),

    #[error("Hook timeout")]
    Timeout,
}

/// Max turns stop hook
pub struct MaxTurnsHook;

#[async_trait::async_trait]
impl StopHook for MaxTurnsHook {
    fn name(&self) -> &str {
        "max_turns"
    }

    async fn execute(
        &self,
        _messages: &[Message],
        _assistant_messages: &[Message],
        context: &StopHookContext,
    ) -> Result<StopHookResult, StopHookError> {
        if let Some(max_turns) = context.max_turns {
            if context.turn >= max_turns {
                return Ok(StopHookResult {
                    prevent_continuation: true,
                    blocking_errors: vec![format!(
                        "Max turns reached: {} >= {}",
                        context.turn, max_turns
                    )],
                    messages: vec![],
                });
            }
        }

        Ok(StopHookResult::default())
    }
}

/// Budget stop hook
pub struct BudgetHook;

#[async_trait::async_trait]
impl StopHook for BudgetHook {
    fn name(&self) -> &str {
        "budget"
    }

    async fn execute(
        &self,
        _messages: &[Message],
        _assistant_messages: &[Message],
        context: &StopHookContext,
    ) -> Result<StopHookResult, StopHookError> {
        if let Some(max_budget) = context.max_budget_usd {
            if context.current_cost_usd >= max_budget {
                return Ok(StopHookResult {
                    prevent_continuation: true,
                    blocking_errors: vec![format!(
                        "Budget exceeded: ${:.2} >= ${:.2}",
                        context.current_cost_usd, max_budget
                    )],
                    messages: vec![],
                });
            }
        }

        Ok(StopHookResult::default())
    }
}

/// Run all stop hooks
pub async fn run_stop_hooks(
    messages: &[Message],
    assistant_messages: &[Message],
    context: &StopHookContext,
    hooks: &[Box<dyn StopHook>],
) -> StopHookResult {
    let mut result = StopHookResult::default();

    for hook in hooks {
        match hook.execute(messages, assistant_messages, context).await {
            Ok(hook_result) => {
                if hook_result.prevent_continuation {
                    result.prevent_continuation = true;
                    result.blocking_errors.extend(hook_result.blocking_errors);
                }
                result.messages.extend(hook_result.messages);
            }
            Err(e) => {
                result
                    .messages
                    .push(format!("Hook {} failed: {}", hook.name(), e));
            }
        }
    }

    result
}

/// Default stop hooks
pub fn default_stop_hooks() -> Vec<Box<dyn StopHook>> {
    vec![Box::new(MaxTurnsHook), Box::new(BudgetHook)]
}
