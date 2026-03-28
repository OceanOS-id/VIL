//! # vil_prompt_optimizer
//!
//! N12 — Auto-Prompt Optimizer: evaluate prompt candidates against test cases,
//! find the best, and generate variations via simple mutations.

pub mod candidate;
pub mod evaluator;
pub mod optimizer;
pub mod strategy;

pub use candidate::{EvaluationResult, PromptCandidate};
pub use evaluator::{KeywordOverlapEvaluator, PromptEvaluator, TestCase};
pub use optimizer::PromptOptimizer;
pub use strategy::OptimizeStrategy;

// VIL integration layer
pub mod handlers;
pub mod pipeline_sse;
pub mod plugin;
pub mod vil_semantic;

pub use plugin::PromptOptimizerPlugin;
pub use vil_semantic::{OptimizeEvent, OptimizeFault, OptimizerState};

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Arc;

    /// Fixed-score evaluator for testing.
    struct FixedScoreEvaluator(f32);

    #[async_trait]
    impl PromptEvaluator for FixedScoreEvaluator {
        async fn evaluate(&self, _template: &str, _tc: &TestCase) -> f32 {
            self.0
        }
    }

    /// Evaluator that scores by template length.
    struct LengthEvaluator;

    #[async_trait]
    impl PromptEvaluator for LengthEvaluator {
        async fn evaluate(&self, template: &str, _tc: &TestCase) -> f32 {
            (template.len() as f32 / 100.0).min(1.0)
        }
    }

    fn test_cases() -> Vec<TestCase> {
        vec![
            TestCase {
                input: "What is AI?".into(),
                expected_output: "Artificial Intelligence".into(),
            },
            TestCase {
                input: "Explain ML".into(),
                expected_output: "Machine Learning".into(),
            },
        ]
    }

    #[tokio::test]
    async fn test_add_candidates() {
        let eval = Arc::new(FixedScoreEvaluator(0.5));
        let mut opt = PromptOptimizer::new(eval, OptimizeStrategy::GridSearch);
        opt.add_candidate("Template A");
        opt.add_candidate("Template B");
        assert_eq!(opt.candidates.len(), 2);
    }

    #[tokio::test]
    async fn test_evaluate_scoring() {
        let eval = Arc::new(FixedScoreEvaluator(0.8));
        let mut opt = PromptOptimizer::new(eval, OptimizeStrategy::GridSearch);
        opt.add_candidate("Prompt 1");
        let results = opt.evaluate_all(&test_cases()).await;
        assert_eq!(results.len(), 1);
        assert!((results[0].1 - 0.8).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_best_selection() {
        let eval = Arc::new(LengthEvaluator);
        let mut opt = PromptOptimizer::new(eval, OptimizeStrategy::GridSearch);
        opt.add_candidate("Short");
        opt.add_candidate("A much longer prompt template that scores higher");
        opt.evaluate_all(&test_cases()).await;
        let best = opt.best().unwrap();
        assert!(best.template.len() > 10);
    }

    #[tokio::test]
    async fn test_empty_candidates() {
        let eval = Arc::new(FixedScoreEvaluator(0.5));
        let mut opt = PromptOptimizer::new(eval, OptimizeStrategy::GridSearch);
        let results = opt.evaluate_all(&test_cases()).await;
        assert!(results.is_empty());
        assert!(opt.best().is_none());
    }

    #[tokio::test]
    async fn test_variation_generation() {
        let eval = Arc::new(FixedScoreEvaluator(0.5));
        let opt = PromptOptimizer::new(eval, OptimizeStrategy::GridSearch);
        let variation = opt.suggest_variation("You are a helpful assistant");
        assert_ne!(variation, "You are a helpful assistant");
        assert!(variation.len() > "You are a helpful assistant".len());
    }

    #[tokio::test]
    async fn test_grid_search_evaluates_all() {
        let eval = Arc::new(FixedScoreEvaluator(1.0));
        let mut opt = PromptOptimizer::new(eval, OptimizeStrategy::GridSearch);
        opt.add_candidate("A");
        opt.add_candidate("B");
        opt.add_candidate("C");
        let results = opt.evaluate_all(&test_cases()).await;
        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn test_random_search_subsets() {
        let eval = Arc::new(FixedScoreEvaluator(1.0));
        let mut opt = PromptOptimizer::new(eval, OptimizeStrategy::RandomSearch);
        opt.add_candidate("A");
        opt.add_candidate("B");
        opt.add_candidate("C");
        opt.add_candidate("D");
        let results = opt.evaluate_all(&test_cases()).await;
        // RandomSearch evaluates every other: indices 0, 2 → 2 candidates.
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_candidate_avg_score() {
        let mut c = PromptCandidate::new("test");
        c.evaluations.push(EvaluationResult {
            test_case: "a".into(),
            score: 0.6,
            notes: None,
        });
        c.evaluations.push(EvaluationResult {
            test_case: "b".into(),
            score: 0.8,
            notes: None,
        });
        assert!((c.avg_score() - 0.7).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_keyword_overlap_evaluator() {
        let eval = KeywordOverlapEvaluator;
        let tc = TestCase {
            input: "What is AI?".into(),
            expected_output: "artificial intelligence systems".into(),
        };
        let score = eval
            .evaluate("Tell me about artificial intelligence", &tc)
            .await;
        assert!(score > 0.0);
    }
}
