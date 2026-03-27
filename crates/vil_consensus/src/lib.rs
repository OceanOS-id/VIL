//! VIL Multi-Model Consensus — parallel inference with voting/fusion.
//!
//! Runs parallel inference across multiple LLM providers and combines results
//! via configurable strategies (majority vote, weighted, best-of-N) for higher accuracy.
//!
//! # Example
//!
//! ```rust,no_run
//! use vil_consensus::{ConsensusEngine, ConsensusStrategy};
//! use vil_llm::ChatMessage;
//! use std::sync::Arc;
//!
//! # async fn demo(providers: Vec<Arc<dyn vil_llm::LlmProvider>>) {
//! let engine = ConsensusEngine::new(providers, ConsensusStrategy::BestOfN);
//! let result = engine.query(&[ChatMessage::user("Explain Rust ownership")]).await.unwrap();
//! println!("Best answer from {}: {}", result.model, result.answer);
//! # }
//! ```

pub mod config;
pub mod engine;
pub mod scorer;
pub mod strategy;

pub use config::ConsensusConfig;
pub use engine::{ConsensusEngine, ConsensusError, ConsensusResult, ProviderResponse};
pub use scorer::{score_response, text_similarity, ResponseScore};
pub use strategy::ConsensusStrategy;

pub mod semantic;
pub mod pipeline_sse;
pub mod handlers;
pub mod plugin;

pub use plugin::ConsensusPlugin;
pub use semantic::{ConsensusEvent, ConsensusFault, ConsensusState};

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Arc;
    use vil_llm::{ChatMessage, ChatResponse, LlmProvider};
    use vil_llm::message::LlmError;

    // -----------------------------------------------------------------------
    // Mock provider
    // -----------------------------------------------------------------------

    struct MockProvider {
        name: String,
        model_name: String,
        response: Result<String, String>,
    }

    impl MockProvider {
        fn ok(name: &str, model: &str, content: &str) -> Arc<dyn LlmProvider> {
            Arc::new(Self {
                name: name.to_string(),
                model_name: model.to_string(),
                response: Ok(content.to_string()),
            })
        }

        fn fail(name: &str, model: &str, err: &str) -> Arc<dyn LlmProvider> {
            Arc::new(Self {
                name: name.to_string(),
                model_name: model.to_string(),
                response: Err(err.to_string()),
            })
        }
    }

    #[async_trait]
    impl LlmProvider for MockProvider {
        async fn chat(&self, _messages: &[ChatMessage]) -> Result<ChatResponse, LlmError> {
            match &self.response {
                Ok(content) => Ok(ChatResponse {
                    content: content.clone(),
                    model: self.model_name.clone(),
                    tool_calls: None,
                    usage: None,
                    finish_reason: None,
                }),
                Err(e) => Err(LlmError::RequestFailed(e.clone())),
            }
        }

        fn model(&self) -> &str {
            &self.model_name
        }

        fn provider_name(&self) -> &str {
            &self.name
        }
    }

    // -----------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------

    fn messages() -> Vec<ChatMessage> {
        vec![ChatMessage::user("test")]
    }

    #[tokio::test]
    async fn test_longest_strategy() {
        let providers = vec![
            MockProvider::ok("a", "model-a", "short"),
            MockProvider::ok("b", "model-b", "this is a much longer response with more detail"),
            MockProvider::ok("c", "model-c", "medium length response"),
        ];

        let engine = ConsensusEngine::new(providers, ConsensusStrategy::Longest);
        let result = engine.query(&messages()).await.unwrap();

        assert_eq!(result.answer, "this is a much longer response with more detail");
        assert_eq!(result.strategy_used, "longest");
    }

    #[tokio::test]
    async fn test_best_of_n_strategy() {
        // Structured response should score higher than plain short text.
        let providers = vec![
            MockProvider::ok("a", "model-a", "ok"),
            MockProvider::ok("b", "model-b", "Here is a detailed answer:\n1. First point about Rust ownership\n2. Second point about borrowing\n- Memory safety guaranteed"),
            MockProvider::ok("c", "model-c", "I don't know I don't know"),
        ];

        let engine = ConsensusEngine::new(providers, ConsensusStrategy::BestOfN);
        let result = engine.query(&messages()).await.unwrap();

        assert_eq!(result.model, "model-b");
        assert_eq!(result.strategy_used, "best_of_n");
    }

    #[tokio::test]
    async fn test_majority_agreement_strategy() {
        // Two similar responses should beat one outlier.
        let providers = vec![
            MockProvider::ok("a", "model-a", "Rust ownership ensures memory safety through the borrow checker"),
            MockProvider::ok("b", "model-b", "Rust ownership ensures memory safety via the borrow checker system"),
            MockProvider::ok("c", "model-c", "Bananas are yellow fruit that grow in tropical climates"),
        ];

        let engine = ConsensusEngine::new(providers, ConsensusStrategy::MajorityAgreement);
        let result = engine.query(&messages()).await.unwrap();

        // The winner should be one of the similar ones, not the outlier.
        assert!(result.answer.contains("Rust ownership"));
        assert_eq!(result.strategy_used, "majority_agreement");
    }

    #[tokio::test]
    async fn test_weighted_strategy() {
        let providers = vec![
            MockProvider::ok("a", "model-a", "short"),
            MockProvider::ok("b", "model-b", "also short"),
        ];

        // Give provider a (index 0) a very high weight.
        let engine = ConsensusEngine::new(providers, ConsensusStrategy::Weighted(vec![100.0, 0.01]));
        let result = engine.query(&messages()).await.unwrap();

        assert_eq!(result.answer, "short");
        assert_eq!(result.strategy_used, "weighted");
    }

    #[tokio::test]
    async fn test_partial_failure() {
        let providers = vec![
            MockProvider::fail("a", "model-a", "connection refused"),
            MockProvider::ok("b", "model-b", "valid response here"),
            MockProvider::fail("c", "model-c", "timeout"),
        ];

        let engine = ConsensusEngine::new(providers, ConsensusStrategy::BestOfN);
        let result = engine.query(&messages()).await.unwrap();

        assert_eq!(result.answer, "valid response here");
        assert_eq!(result.all_responses.len(), 3);

        let errors: Vec<_> = result.all_responses.iter().filter(|r| r.error.is_some()).collect();
        assert_eq!(errors.len(), 2);
    }

    #[tokio::test]
    async fn test_all_fail() {
        let providers = vec![
            MockProvider::fail("a", "model-a", "error 1"),
            MockProvider::fail("b", "model-b", "error 2"),
        ];

        let engine = ConsensusEngine::new(providers, ConsensusStrategy::BestOfN);
        let result = engine.query(&messages()).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ConsensusError::AllProvidersFailed(errs) => {
                assert_eq!(errs.len(), 2);
            }
            other => panic!("expected AllProvidersFailed, got {:?}", other),
        }
    }
}
