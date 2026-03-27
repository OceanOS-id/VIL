use dashmap::DashMap;
use serde::{Serialize, Deserialize};
use std::fmt;

/// Cost data for a model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCost {
    pub model: String,
    pub cost_per_1k_input: f64,
    pub cost_per_1k_output: f64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cost_usd: f64,
}

/// Budget for an API key or team.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub key: String,
    pub limit_usd: f64,
    pub spent_usd: f64,
}

/// Error when budget is exceeded.
#[derive(Debug, Clone)]
pub struct BudgetExceeded {
    pub key: String,
    pub limit_usd: f64,
    pub spent_usd: f64,
}

impl fmt::Display for BudgetExceeded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "budget exceeded for '{}': limit=${:.4}, spent=${:.4}",
            self.key, self.limit_usd, self.spent_usd
        )
    }
}

impl std::error::Error for BudgetExceeded {}

/// Per-model cost tracking with budget enforcement.
pub struct CostTracker {
    models: DashMap<String, ModelCost>,
    budgets: DashMap<String, Budget>,
}

impl CostTracker {
    pub fn new() -> Self {
        Self {
            models: DashMap::new(),
            budgets: DashMap::new(),
        }
    }

    /// Set pricing for a model (cost per 1K tokens).
    pub fn set_model_pricing(&self, model: &str, input_per_1k: f64, output_per_1k: f64) {
        self.models
            .entry(model.to_string())
            .and_modify(|c| {
                c.cost_per_1k_input = input_per_1k;
                c.cost_per_1k_output = output_per_1k;
            })
            .or_insert(ModelCost {
                model: model.to_string(),
                cost_per_1k_input: input_per_1k,
                cost_per_1k_output: output_per_1k,
                total_input_tokens: 0,
                total_output_tokens: 0,
                total_cost_usd: 0.0,
            });
    }

    /// Record token usage for a model and return the cost in USD.
    pub fn record_usage(&self, model: &str, input_tokens: u32, output_tokens: u32) -> f64 {
        let mut entry = self.models.entry(model.to_string()).or_insert(ModelCost {
            model: model.to_string(),
            cost_per_1k_input: 0.0,
            cost_per_1k_output: 0.0,
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cost_usd: 0.0,
        });
        let cost_data = entry.value_mut();
        let input_cost = (input_tokens as f64 / 1000.0) * cost_data.cost_per_1k_input;
        let output_cost = (output_tokens as f64 / 1000.0) * cost_data.cost_per_1k_output;
        let total = input_cost + output_cost;
        cost_data.total_input_tokens += input_tokens as u64;
        cost_data.total_output_tokens += output_tokens as u64;
        cost_data.total_cost_usd += total;
        total
    }

    /// Set a spending budget for a key (API key or team name).
    pub fn set_budget(&self, key: &str, limit_usd: f64) {
        self.budgets
            .entry(key.to_string())
            .and_modify(|b| b.limit_usd = limit_usd)
            .or_insert(Budget {
                key: key.to_string(),
                limit_usd,
                spent_usd: 0.0,
            });
    }

    /// Record spending against a budget key.
    pub fn record_budget_spend(&self, key: &str, amount_usd: f64) {
        if let Some(mut budget) = self.budgets.get_mut(key) {
            budget.spent_usd += amount_usd;
        }
    }

    /// Check if a budget key is still under limit.
    pub fn check_budget(&self, key: &str) -> Result<(), BudgetExceeded> {
        match self.budgets.get(key) {
            Some(budget) => {
                if budget.spent_usd >= budget.limit_usd {
                    Err(BudgetExceeded {
                        key: key.to_string(),
                        limit_usd: budget.limit_usd,
                        spent_usd: budget.spent_usd,
                    })
                } else {
                    Ok(())
                }
            }
            None => Ok(()), // no budget set = unlimited
        }
    }

    /// Get cost data for a specific model.
    pub fn get_cost(&self, model: &str) -> Option<ModelCost> {
        self.models.get(model).map(|e| e.value().clone())
    }

    /// Total cost across all models.
    pub fn total_cost(&self) -> f64 {
        self.models.iter().map(|e| e.value().total_cost_usd).sum()
    }

    /// Return the cheapest model by cost-per-1K-input.
    pub fn cheapest_model(&self) -> Option<String> {
        self.models
            .iter()
            .filter(|e| e.value().cost_per_1k_input > 0.0 || e.value().cost_per_1k_output > 0.0)
            .min_by(|a, b| {
                let cost_a = a.value().cost_per_1k_input + a.value().cost_per_1k_output;
                let cost_b = b.value().cost_per_1k_input + b.value().cost_per_1k_output;
                cost_a.partial_cmp(&cost_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|e| e.key().clone())
    }
}

impl Default for CostTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_pricing_and_record_usage() {
        let tracker = CostTracker::new();
        tracker.set_model_pricing("gpt-4", 0.03, 0.06);

        let cost = tracker.record_usage("gpt-4", 1000, 500);
        // input: 1000/1000 * 0.03 = 0.03
        // output: 500/1000 * 0.06 = 0.03
        assert!((cost - 0.06).abs() < 0.0001);

        let data = tracker.get_cost("gpt-4").unwrap();
        assert_eq!(data.total_input_tokens, 1000);
        assert_eq!(data.total_output_tokens, 500);
        assert!((data.total_cost_usd - 0.06).abs() < 0.0001);
    }

    #[test]
    fn test_budget_enforcement() {
        let tracker = CostTracker::new();
        tracker.set_budget("team-a", 1.00);
        assert!(tracker.check_budget("team-a").is_ok());

        tracker.record_budget_spend("team-a", 0.80);
        assert!(tracker.check_budget("team-a").is_ok());

        tracker.record_budget_spend("team-a", 0.25);
        assert!(tracker.check_budget("team-a").is_err());
    }

    #[test]
    fn test_no_budget_means_unlimited() {
        let tracker = CostTracker::new();
        assert!(tracker.check_budget("unknown-key").is_ok());
    }

    #[test]
    fn test_total_cost() {
        let tracker = CostTracker::new();
        tracker.set_model_pricing("gpt-4", 0.03, 0.06);
        tracker.set_model_pricing("claude", 0.01, 0.03);
        tracker.record_usage("gpt-4", 1000, 0);
        tracker.record_usage("claude", 1000, 0);
        let total = tracker.total_cost();
        assert!((total - 0.04).abs() < 0.0001);
    }

    #[test]
    fn test_cheapest_model() {
        let tracker = CostTracker::new();
        tracker.set_model_pricing("expensive", 0.10, 0.20);
        tracker.set_model_pricing("cheap", 0.001, 0.002);
        assert_eq!(tracker.cheapest_model(), Some("cheap".to_string()));
    }
}
