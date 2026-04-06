//! Budget tracking for query execution.
//!
//! Tracks turns, tokens, and USD costs.

use serde::{Deserialize, Serialize};

/// Budget tracker
#[derive(Debug, Clone)]
pub struct BudgetTracker {
    /// Max turns allowed
    pub max_turns: Option<u32>,

    /// Max budget in USD
    pub max_budget_usd: Option<f64>,

    /// Current turn
    pub current_turn: u32,

    /// Current cost
    pub current_cost_usd: f64,

    /// Token usage
    pub total_tokens: u64,
}

impl BudgetTracker {
    pub fn new(max_turns: Option<u32>, max_budget_usd: Option<f64>) -> Self {
        Self {
            max_turns,
            max_budget_usd,
            current_turn: 0,
            current_cost_usd: 0.0,
            total_tokens: 0,
        }
    }

    /// Increment turn
    pub fn increment_turn(&mut self) {
        self.current_turn += 1;
    }

    /// Add cost
    pub fn add_cost(&mut self, cost_usd: f64, tokens: u64) {
        self.current_cost_usd += cost_usd;
        self.total_tokens += tokens;
    }

    /// Check if max turns reached
    pub fn is_max_turns_reached(&self) -> bool {
        self.max_turns
            .map(|max| self.current_turn >= max)
            .unwrap_or(false)
    }

    /// Check if budget exceeded
    pub fn is_budget_exceeded(&self) -> bool {
        self.max_budget_usd
            .map(|max| self.current_cost_usd >= max)
            .unwrap_or(false)
    }

    /// Get remaining turns
    pub fn remaining_turns(&self) -> Option<u32> {
        self.max_turns
            .map(|max| max.saturating_sub(self.current_turn))
    }

    /// Get remaining budget
    pub fn remaining_budget(&self) -> Option<f64> {
        self.max_budget_usd.map(|max| max - self.current_cost_usd)
    }

    /// Get usage summary
    pub fn summary(&self) -> BudgetSummary {
        BudgetSummary {
            turns_used: self.current_turn,
            max_turns: self.max_turns,
            cost_usd: self.current_cost_usd,
            max_budget_usd: self.max_budget_usd,
            total_tokens: self.total_tokens,
        }
    }
}

impl Default for BudgetTracker {
    fn default() -> Self {
        Self::new(None, None)
    }
}

/// Budget summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetSummary {
    pub turns_used: u32,
    pub max_turns: Option<u32>,
    pub cost_usd: f64,
    pub max_budget_usd: Option<f64>,
    pub total_tokens: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_tracker() {
        let mut tracker = BudgetTracker::new(Some(10), Some(5.0));

        tracker.increment_turn();
        tracker.add_cost(0.5, 1000);

        assert_eq!(tracker.current_turn, 1);
        assert_eq!(tracker.current_cost_usd, 0.5);
        assert!(!tracker.is_max_turns_reached());
        assert!(!tracker.is_budget_exceeded());

        assert_eq!(tracker.remaining_turns(), Some(9));
        assert_eq!(tracker.remaining_budget(), Some(4.5));
    }
}
