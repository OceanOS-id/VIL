use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use vil_macros::VilAiState;

use crate::budget::{Budget, BudgetExceeded};

/// Usage stats for a single model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsage {
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub requests: u64,
    pub cost_usd: f64,
}

/// Pricing configuration for a model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    pub model: String,
    /// Cost per 1,000 input tokens in USD.
    pub input_per_1k: f64,
    /// Cost per 1,000 output tokens in USD.
    pub output_per_1k: f64,
}

/// Per-model cost breakdown in a report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCostEntry {
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub requests: u64,
    pub cost_usd: f64,
}

/// Aggregate cost report.
#[derive(Debug, Clone, Serialize, Deserialize, VilAiState)]
pub struct CostReport {
    pub models: Vec<ModelCostEntry>,
    pub total_cost_usd: f64,
    pub total_requests: u64,
}

/// Tracks LLM usage and costs across multiple models.
pub struct CostTracker {
    pub models: DashMap<String, ModelUsage>,
    pricing: DashMap<String, ModelPricing>,
    budgets: DashMap<String, Budget>,
}

impl CostTracker {
    pub fn new() -> Self {
        Self {
            models: DashMap::new(),
            pricing: DashMap::new(),
            budgets: DashMap::new(),
        }
    }

    /// Set pricing for a model.
    pub fn set_pricing(&self, pricing: ModelPricing) {
        self.pricing.insert(pricing.model.clone(), pricing);
    }

    /// Record usage for a model. Automatically calculates cost if pricing is set.
    pub fn record(&self, model: &str, input_tokens: u64, output_tokens: u64) {
        let cost = self.calculate_cost(model, input_tokens, output_tokens);

        self.models
            .entry(model.to_string())
            .and_modify(|usage| {
                usage.input_tokens += input_tokens;
                usage.output_tokens += output_tokens;
                usage.requests += 1;
                usage.cost_usd += cost;
            })
            .or_insert_with(|| ModelUsage {
                model: model.to_string(),
                input_tokens,
                output_tokens,
                requests: 1,
                cost_usd: cost,
            });
    }

    fn calculate_cost(&self, model: &str, input_tokens: u64, output_tokens: u64) -> f64 {
        if let Some(p) = self.pricing.get(model) {
            let input_cost = (input_tokens as f64 / 1000.0) * p.input_per_1k;
            let output_cost = (output_tokens as f64 / 1000.0) * p.output_per_1k;
            input_cost + output_cost
        } else {
            0.0
        }
    }

    /// Set a budget for a given key.
    pub fn set_budget(&self, budget: Budget) {
        self.budgets.insert(budget.key.clone(), budget);
    }

    /// Check if budget for the given key is exceeded.
    pub fn check_budget(&self, key: &str) -> Result<(), BudgetExceeded> {
        if let Some(budget) = self.budgets.get(key) {
            let total_cost = self.total_cost();
            if total_cost > budget.limit_usd {
                return Err(BudgetExceeded {
                    key: key.to_string(),
                    limit_usd: budget.limit_usd,
                    current_usd: total_cost,
                });
            }
        }
        Ok(())
    }

    /// Total cost across all models.
    pub fn total_cost(&self) -> f64 {
        self.models.iter().map(|e| e.value().cost_usd).sum()
    }

    /// Generate a cost report with per-model breakdown.
    pub fn cost_report(&self) -> CostReport {
        let mut models = Vec::new();
        let mut total_cost = 0.0;
        let mut total_requests = 0u64;

        for entry in self.models.iter() {
            let u = entry.value();
            total_cost += u.cost_usd;
            total_requests += u.requests;
            models.push(ModelCostEntry {
                model: u.model.clone(),
                input_tokens: u.input_tokens,
                output_tokens: u.output_tokens,
                requests: u.requests,
                cost_usd: u.cost_usd,
            });
        }

        models.sort_by(|a, b| b.cost_usd.partial_cmp(&a.cost_usd).unwrap());

        CostReport {
            models,
            total_cost_usd: total_cost,
            total_requests,
        }
    }
}

impl Default for CostTracker {
    fn default() -> Self {
        Self::new()
    }
}
