//! Main context optimizer.
//!
//! Orchestrates deduplication, scoring, and budget-fitting to produce a
//! compressed context that maximizes information within token limits.

use vil_tokenizer::TokenCounter;

use crate::budget::TokenBudget;
use crate::dedup::deduplicate;
use crate::scorer::{score_chunks, ChunkScore, ScoringWeights};
use crate::strategy::OptimizeStrategy;

/// The main context optimizer.
pub struct ContextOptimizer {
    counter: TokenCounter,
    budget: TokenBudget,
    strategy: OptimizeStrategy,
    weights: ScoringWeights,
}

/// Result of context optimization.
#[derive(Debug, Clone)]
pub struct OptimizedContext {
    /// The selected chunks in final order.
    pub chunks: Vec<String>,
    /// Total tokens used by selected chunks.
    pub total_tokens: usize,
    /// Number of chunks in the original input.
    pub original_count: usize,
    /// Number of chunks after optimization.
    pub final_count: usize,
    /// Tokens saved compared to including all chunks.
    pub tokens_saved: usize,
    /// Compression ratio: final_tokens / original_tokens (lower = more compression).
    pub compression_ratio: f32,
}

impl ContextOptimizer {
    /// Create a new optimizer with a token budget.
    pub fn new(budget: TokenBudget) -> Self {
        Self {
            counter: TokenCounter::gpt4(),
            budget,
            strategy: OptimizeStrategy::default(),
            weights: ScoringWeights::default(),
        }
    }

    /// Set the optimization strategy.
    pub fn strategy(mut self, s: OptimizeStrategy) -> Self {
        self.strategy = s;
        self
    }

    /// Set scoring weights.
    pub fn weights(mut self, w: ScoringWeights) -> Self {
        self.weights = w;
        self
    }

    /// Set a custom token counter.
    pub fn counter(mut self, c: TokenCounter) -> Self {
        self.counter = c;
        self
    }

    /// Optimize a list of chunks with retrieval scores to fit token budget.
    ///
    /// Each input entry is `(chunk_text, retrieval_score)`.
    pub fn optimize(&self, chunks: &[(String, f32)]) -> OptimizedContext {
        let original_count = chunks.len();
        let original_tokens: usize = chunks.iter().map(|(t, _)| self.counter.count(t)).sum();

        if chunks.is_empty() {
            return OptimizedContext {
                chunks: Vec::new(),
                total_tokens: 0,
                original_count: 0,
                final_count: 0,
                tokens_saved: 0,
                compression_ratio: 1.0,
            };
        }

        let selected = match &self.strategy {
            OptimizeStrategy::TopK(k) => self.top_k(chunks, *k),
            OptimizeStrategy::BudgetFit => self.budget_fit(chunks),
            OptimizeStrategy::DedupAndFit { dedup_threshold } => {
                self.dedup_and_fit(chunks, *dedup_threshold)
            }
            OptimizeStrategy::Full { dedup_threshold } => {
                self.full_optimize(chunks, *dedup_threshold)
            }
        };

        let total_tokens: usize = selected.iter().map(|t| self.counter.count(t)).sum();
        let final_count = selected.len();
        let tokens_saved = original_tokens.saturating_sub(total_tokens);
        let compression_ratio = if original_tokens == 0 {
            1.0
        } else {
            total_tokens as f32 / original_tokens as f32
        };

        OptimizedContext {
            chunks: selected,
            total_tokens,
            original_count,
            final_count,
            tokens_saved,
            compression_ratio,
        }
    }

    /// Keep top-K chunks by score.
    fn top_k(&self, chunks: &[(String, f32)], k: usize) -> Vec<String> {
        let count_fn = |text: &str| self.counter.count(text);
        let mut scored = score_chunks(chunks, &self.weights, &count_fn);
        scored.sort_by(|a, b| b.combined.partial_cmp(&a.combined).unwrap());
        scored.into_iter().take(k).map(|s| s.text).collect()
    }

    /// Greedily fit chunks into the token budget, ordered by score.
    fn budget_fit(&self, chunks: &[(String, f32)]) -> Vec<String> {
        let count_fn = |text: &str| self.counter.count(text);
        let mut scored = score_chunks(chunks, &self.weights, &count_fn);
        scored.sort_by(|a, b| b.combined.partial_cmp(&a.combined).unwrap());
        self.greedy_fill(scored)
    }

    /// Deduplicate then budget-fit.
    fn dedup_and_fit(&self, chunks: &[(String, f32)], threshold: f32) -> Vec<String> {
        let texts: Vec<String> = chunks.iter().map(|(t, _)| t.clone()).collect();
        let keep_indices = deduplicate(&texts, threshold);

        let deduped: Vec<(String, f32)> = keep_indices
            .iter()
            .map(|&i| chunks[i].clone())
            .collect();

        self.budget_fit(&deduped)
    }

    /// Full pipeline: dedup -> score -> sort -> greedy fill.
    fn full_optimize(&self, chunks: &[(String, f32)], threshold: f32) -> Vec<String> {
        let texts: Vec<String> = chunks.iter().map(|(t, _)| t.clone()).collect();
        let keep_indices = deduplicate(&texts, threshold);

        let deduped: Vec<(String, f32)> = keep_indices
            .iter()
            .map(|&i| chunks[i].clone())
            .collect();

        let count_fn = |text: &str| self.counter.count(text);
        let mut scored = score_chunks(&deduped, &self.weights, &count_fn);
        scored.sort_by(|a, b| b.combined.partial_cmp(&a.combined).unwrap());

        self.greedy_fill(scored)
    }

    /// Greedily add chunks (already sorted by score desc) until budget is exhausted.
    fn greedy_fill(&self, scored: Vec<ChunkScore>) -> Vec<String> {
        let budget = self.budget.available;
        let mut used = 0usize;
        let mut result = Vec::new();

        for chunk in scored {
            if used + chunk.tokens <= budget {
                used += chunk.tokens;
                result.push(chunk.text);
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Simple word-count estimator for tests (avoids BPE vocab loading issues).
    fn make_test_chunks(texts: &[&str], scores: &[f32]) -> Vec<(String, f32)> {
        texts
            .iter()
            .zip(scores.iter())
            .map(|(t, s)| (t.to_string(), *s))
            .collect()
    }

    #[test]
    fn test_empty_input() {
        let budget = TokenBudget::new(8000);
        let optimizer = ContextOptimizer::new(budget);
        let result = optimizer.optimize(&[]);
        assert_eq!(result.original_count, 0);
        assert_eq!(result.final_count, 0);
        assert_eq!(result.total_tokens, 0);
        assert_eq!(result.compression_ratio, 1.0);
    }

    #[test]
    fn test_top_k_strategy() {
        let budget = TokenBudget::new(100_000);
        let optimizer = ContextOptimizer::new(budget)
            .strategy(OptimizeStrategy::TopK(2));

        let chunks = make_test_chunks(
            &["low score chunk", "medium score chunk", "high score chunk"],
            &[0.1, 0.5, 0.9],
        );

        let result = optimizer.optimize(&chunks);
        assert_eq!(result.final_count, 2);
    }

    #[test]
    fn test_budget_fit_respects_limit() {
        // Use a very small budget so not all chunks fit
        let budget = TokenBudget::new(20).system_tokens(0).response_tokens(0);
        let optimizer = ContextOptimizer::new(budget)
            .strategy(OptimizeStrategy::BudgetFit);

        let chunks = make_test_chunks(
            &[
                "a b c d e f g h i j k l m n o p q r s t",
                "u v w x y z a b c d e f g h i j k l m n",
                "short",
            ],
            &[0.5, 0.5, 0.9],
        );

        let result = optimizer.optimize(&chunks);
        // Should not include everything — budget is only 20 tokens
        assert!(result.total_tokens <= 20);
    }

    #[test]
    fn test_dedup_and_fit() {
        let budget = TokenBudget::new(100_000);
        let optimizer = ContextOptimizer::new(budget)
            .strategy(OptimizeStrategy::DedupAndFit {
                dedup_threshold: 0.8,
            });

        let chunks = make_test_chunks(
            &[
                "the quick brown fox jumps over the lazy dog",
                "the quick brown fox jumps over the lazy dog", // exact duplicate
                "completely different unrelated text here",
            ],
            &[0.8, 0.8, 0.7],
        );

        let result = optimizer.optimize(&chunks);
        assert_eq!(result.original_count, 3);
        // The duplicate should be removed
        assert!(result.final_count <= 2);
    }

    #[test]
    fn test_full_optimization() {
        let budget = TokenBudget::new(100_000);
        let optimizer = ContextOptimizer::new(budget)
            .strategy(OptimizeStrategy::Full {
                dedup_threshold: 0.8,
            });

        let chunks = make_test_chunks(
            &[
                "rust is a systems programming language",
                "rust is a systems programming language", // duplicate
                "python is great for data science",
                "javascript runs in browsers",
            ],
            &[0.9, 0.9, 0.7, 0.6],
        );

        let result = optimizer.optimize(&chunks);
        assert_eq!(result.original_count, 4);
        assert!(result.final_count <= 3); // at least the duplicate removed
        assert!(result.tokens_saved > 0);
    }

    #[test]
    fn test_compression_ratio() {
        let budget = TokenBudget::new(100_000);
        let optimizer = ContextOptimizer::new(budget)
            .strategy(OptimizeStrategy::TopK(1));

        let chunks = make_test_chunks(
            &["chunk one text here", "chunk two text here", "chunk three text here"],
            &[0.9, 0.5, 0.3],
        );

        let result = optimizer.optimize(&chunks);
        assert!(result.compression_ratio > 0.0);
        assert!(result.compression_ratio <= 1.0);
    }

    #[test]
    fn test_custom_weights() {
        let budget = TokenBudget::new(100_000);
        let weights = ScoringWeights {
            relevance: 1.0,
            recency: 0.0,
            uniqueness: 0.0,
        };
        let optimizer = ContextOptimizer::new(budget)
            .weights(weights)
            .strategy(OptimizeStrategy::TopK(1));

        let chunks = make_test_chunks(
            &["low relevance", "high relevance"],
            &[0.1, 0.9],
        );

        let result = optimizer.optimize(&chunks);
        assert_eq!(result.final_count, 1);
        assert_eq!(result.chunks[0], "high relevance");
    }
}
