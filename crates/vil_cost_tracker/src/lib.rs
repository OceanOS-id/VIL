//! # vil_cost_tracker (I09)
//!
//! Rate Limiter + Cost Tracker — LLM usage tracking with budget enforcement.
//!
//! Tracks per-model token usage and costs, enforces spending budgets,
//! and generates cost breakdown reports.

pub mod budget;
pub mod report;
pub mod tracker;
pub mod semantic;
pub mod handlers;
pub mod plugin;
pub mod pipeline_sse;

pub use budget::{Budget, BudgetExceeded, BudgetPeriod};
pub use tracker::{CostReport, CostTracker, ModelCostEntry, ModelPricing, ModelUsage};
pub use plugin::CostTrackerPlugin;
pub use semantic::{CostEvent, CostFault, CostFaultType, CostState};

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_tracker() -> CostTracker {
        let t = CostTracker::new();
        t.set_pricing(ModelPricing {
            model: "gpt-4".into(),
            input_per_1k: 0.03,
            output_per_1k: 0.06,
        });
        t.set_pricing(ModelPricing {
            model: "gpt-3.5".into(),
            input_per_1k: 0.001,
            output_per_1k: 0.002,
        });
        t
    }

    #[test]
    fn test_record_usage() {
        let t = setup_tracker();
        t.record("gpt-4", 1000, 500);
        let usage = t.models.get("gpt-4").unwrap();
        assert_eq!(usage.input_tokens, 1000);
        assert_eq!(usage.output_tokens, 500);
        assert_eq!(usage.requests, 1);
    }

    #[test]
    fn test_cost_calculation() {
        let t = setup_tracker();
        // 1000 input tokens * $0.03/1k = $0.03
        // 500 output tokens * $0.06/1k = $0.03
        // Total = $0.06
        t.record("gpt-4", 1000, 500);
        let usage = t.models.get("gpt-4").unwrap();
        assert!((usage.cost_usd - 0.06).abs() < 1e-10);
    }

    #[test]
    fn test_multiple_records() {
        let t = setup_tracker();
        t.record("gpt-4", 1000, 500);
        t.record("gpt-4", 2000, 1000);
        let usage = t.models.get("gpt-4").unwrap();
        assert_eq!(usage.input_tokens, 3000);
        assert_eq!(usage.output_tokens, 1500);
        assert_eq!(usage.requests, 2);
    }

    #[test]
    fn test_budget_enforcement_ok() {
        let t = setup_tracker();
        t.set_budget(Budget::new("default", 1.0, BudgetPeriod::Total));
        t.record("gpt-4", 1000, 500); // $0.06
        assert!(t.check_budget("default").is_ok());
    }

    #[test]
    fn test_budget_enforcement_exceeded() {
        let t = setup_tracker();
        t.set_budget(Budget::new("default", 0.01, BudgetPeriod::Total));
        t.record("gpt-4", 1000, 500); // $0.06 > $0.01
        assert!(t.check_budget("default").is_err());
    }

    #[test]
    fn test_pricing_update() {
        let t = CostTracker::new();
        t.set_pricing(ModelPricing {
            model: "gpt-4".into(),
            input_per_1k: 0.03,
            output_per_1k: 0.06,
        });
        t.record("gpt-4", 1000, 0);
        let cost1 = t.models.get("gpt-4").unwrap().cost_usd;

        // Update pricing
        t.set_pricing(ModelPricing {
            model: "gpt-4".into(),
            input_per_1k: 0.01,
            output_per_1k: 0.02,
        });
        t.record("gpt-4", 1000, 0);
        let cost2 = t.models.get("gpt-4").unwrap().cost_usd;

        // Second record should use new pricing: $0.03 + $0.01 = $0.04
        assert!((cost2 - 0.04).abs() < 1e-10);
        assert!(cost2 < cost1 * 2.0);
    }

    #[test]
    fn test_report_generation() {
        let t = setup_tracker();
        t.record("gpt-4", 1000, 500);
        t.record("gpt-3.5", 2000, 1000);
        let report = t.cost_report();
        assert_eq!(report.models.len(), 2);
        assert_eq!(report.total_requests, 2);
        assert!(report.total_cost_usd > 0.0);
    }

    #[test]
    fn test_multiple_models() {
        let t = setup_tracker();
        t.record("gpt-4", 1000, 500);
        t.record("gpt-3.5", 5000, 2000);
        assert!(t.models.contains_key("gpt-4"));
        assert!(t.models.contains_key("gpt-3.5"));
        // gpt-3.5 should be much cheaper
        let gpt4_cost = t.models.get("gpt-4").unwrap().cost_usd;
        let gpt35_cost = t.models.get("gpt-3.5").unwrap().cost_usd;
        assert!(gpt4_cost > gpt35_cost);
    }

    #[test]
    fn test_no_pricing_zero_cost() {
        let t = CostTracker::new();
        t.record("unknown-model", 1000, 500);
        let usage = t.models.get("unknown-model").unwrap();
        assert_eq!(usage.cost_usd, 0.0);
    }

    #[test]
    fn test_budget_nonexistent_key() {
        let t = CostTracker::new();
        // No budget set for "foo" — should pass
        assert!(t.check_budget("foo").is_ok());
    }
}
