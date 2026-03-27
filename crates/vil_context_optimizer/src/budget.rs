//! Token budget management.
//!
//! Partitions a total token budget into system, response, and available context regions.

use serde::{Deserialize, Serialize};

/// Represents a token budget split into reserved and available regions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBudget {
    /// Total token budget (model context window).
    pub total: usize,
    /// Tokens reserved for system prompt.
    pub system_reserved: usize,
    /// Tokens reserved for model response.
    pub response_reserved: usize,
    /// Tokens available for context = total - system - response.
    pub available: usize,
}

impl TokenBudget {
    /// Create a new budget with default reserves (system=500, response=1000).
    pub fn new(total: usize) -> Self {
        let system_reserved = 500;
        let response_reserved = 1000;
        let available = total.saturating_sub(system_reserved + response_reserved);
        Self {
            total,
            system_reserved,
            response_reserved,
            available,
        }
    }

    /// Set system token reserve.
    pub fn system_tokens(mut self, n: usize) -> Self {
        self.system_reserved = n;
        self.recalc();
        self
    }

    /// Set response token reserve.
    pub fn response_tokens(mut self, n: usize) -> Self {
        self.response_reserved = n;
        self.recalc();
        self
    }

    fn recalc(&mut self) {
        self.available = self
            .total
            .saturating_sub(self.system_reserved + self.response_reserved);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_budget() {
        let b = TokenBudget::new(8000);
        assert_eq!(b.total, 8000);
        assert_eq!(b.system_reserved, 500);
        assert_eq!(b.response_reserved, 1000);
        assert_eq!(b.available, 6500);
    }

    #[test]
    fn test_custom_reserves() {
        let b = TokenBudget::new(8000)
            .system_tokens(1000)
            .response_tokens(2000);
        assert_eq!(b.available, 5000);
    }

    #[test]
    fn test_saturating_budget() {
        let b = TokenBudget::new(100)
            .system_tokens(80)
            .response_tokens(80);
        assert_eq!(b.available, 0);
    }

    #[test]
    fn test_zero_budget() {
        let b = TokenBudget::new(0);
        assert_eq!(b.available, 0);
    }
}
