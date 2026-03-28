//! # vil_model_serving (D19)
//!
//! Differential model serving — A/B test model versions with traffic splitting,
//! per-variant quality metrics, and auto-promote/rollback policies.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use vil_model_serving::{ModelServer, ModelVariant, PromotionPolicy};
//!
//! // Create variants (each wrapping an LlmProvider)
//! // let v1 = ModelVariant::new("gpt4-stable", provider_a, 0.8, 1);
//! // let v2 = ModelVariant::new("gpt4-canary", provider_b, 0.2, 2);
//! //
//! // let server = ModelServer::new(
//! //     vec![v1, v2],
//! //     PromotionPolicy::AutoPromote { min_requests: 100, min_quality: 0.9 },
//! // );
//! //
//! // let result = server.serve(&messages).await?;
//! // server.record_quality(&result.variant_name, 0.95);
//! // server.apply_policy();
//! ```

pub mod handlers;
pub mod metrics;
pub mod pipeline_sse;
pub mod plugin;
pub mod policy;
pub mod semantic;
pub mod serving;
pub mod variant;

pub use metrics::VariantMetrics;
pub use plugin::ModelServingPlugin;
pub use policy::PromotionPolicy;
pub use semantic::{ServingEvent, ServingFault, ServingState};
pub use serving::{ModelServer, ServeError, ServeResult};
pub use variant::ModelVariant;

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Arc;
    use vil_llm::message::LlmError;
    use vil_llm::{ChatMessage, ChatResponse, LlmProvider};

    /// Mock provider that returns a fixed response.
    struct MockProvider {
        name: String,
        response: String,
    }

    impl MockProvider {
        fn new(name: &str, response: &str) -> Self {
            Self {
                name: name.to_string(),
                response: response.to_string(),
            }
        }
    }

    #[async_trait]
    impl LlmProvider for MockProvider {
        async fn chat(&self, _messages: &[ChatMessage]) -> Result<ChatResponse, LlmError> {
            Ok(ChatResponse {
                content: self.response.clone(),
                model: self.name.clone(),
                tool_calls: None,
                usage: None,
                finish_reason: Some("stop".into()),
            })
        }

        fn model(&self) -> &str {
            &self.name
        }

        fn provider_name(&self) -> &str {
            "mock"
        }
    }

    fn make_variants() -> Vec<ModelVariant> {
        vec![
            ModelVariant::new(
                "model-a",
                Arc::new(MockProvider::new("model-a", "response-a")),
                0.8,
                1,
            ),
            ModelVariant::new(
                "model-b",
                Arc::new(MockProvider::new("model-b", "response-b")),
                0.2,
                2,
            ),
        ]
    }

    #[tokio::test]
    async fn test_weighted_selection_respects_weights() {
        let server = ModelServer::new(make_variants(), PromotionPolicy::Manual);
        let msgs = vec![ChatMessage::user("hello")];

        let mut a_count = 0u64;
        let mut b_count = 0u64;
        for _ in 0..100 {
            let r = server.serve(&msgs).await.unwrap();
            if r.variant_name == "model-a" {
                a_count += 1;
            } else {
                b_count += 1;
            }
        }
        // model-a has 80% weight, model-b has 20%
        assert!(
            a_count > b_count,
            "model-a ({a_count}) should be selected more than model-b ({b_count})"
        );
    }

    #[tokio::test]
    async fn test_promote_sets_100_percent() {
        let server = ModelServer::new(make_variants(), PromotionPolicy::Manual);
        server.promote("model-b");

        let msgs = vec![ChatMessage::user("test")];
        for _ in 0..20 {
            let r = server.serve(&msgs).await.unwrap();
            assert_eq!(r.variant_name, "model-b");
        }
    }

    #[tokio::test]
    async fn test_rollback_removes_variant() {
        let server = ModelServer::new(make_variants(), PromotionPolicy::Manual);
        assert_eq!(server.variant_count(), 2);

        server.rollback("model-a");
        assert_eq!(server.variant_count(), 1);

        let msgs = vec![ChatMessage::user("test")];
        let r = server.serve(&msgs).await.unwrap();
        assert_eq!(r.variant_name, "model-b");
    }

    #[tokio::test]
    async fn test_metrics_recording() {
        let server = ModelServer::new(make_variants(), PromotionPolicy::Manual);
        let msgs = vec![ChatMessage::user("hi")];

        let r = server.serve(&msgs).await.unwrap();
        server.record_quality(&r.variant_name, 0.9);

        let metrics = server.get_metrics();
        let served = metrics
            .iter()
            .find(|(name, _)| name == &r.variant_name)
            .unwrap();
        assert_eq!(served.1.requests, 1);
        assert!(served.1.avg_quality_score > 0.0);
    }

    #[tokio::test]
    async fn test_auto_promote_policy() {
        let server = ModelServer::new(
            make_variants(),
            PromotionPolicy::AutoPromote {
                min_requests: 5,
                min_quality: 0.8,
            },
        );

        let msgs = vec![ChatMessage::user("x")];

        // Serve enough requests and record quality for model-b
        // First promote model-b to get all traffic to it
        server.promote("model-b");
        for _ in 0..6 {
            let r = server.serve(&msgs).await.unwrap();
            server.record_quality(&r.variant_name, 0.95);
        }

        // Apply policy — model-b should remain promoted (already at 100%)
        server.apply_policy();

        // Verify model-b is the only one serving
        let r = server.serve(&msgs).await.unwrap();
        assert_eq!(r.variant_name, "model-b");
    }

    #[tokio::test]
    async fn test_serve_with_mock_providers() {
        let server = ModelServer::new(
            vec![ModelVariant::new(
                "solo",
                Arc::new(MockProvider::new("solo", "solo-response")),
                1.0,
                1,
            )],
            PromotionPolicy::Manual,
        );

        let msgs = vec![ChatMessage::user("hello")];
        let r = server.serve(&msgs).await.unwrap();
        assert_eq!(r.content, "solo-response");
        assert_eq!(r.variant_name, "solo");
        assert_eq!(r.version, 1);
    }

    #[tokio::test]
    async fn test_variant_with_zero_weight_never_selected() {
        let variants = vec![
            ModelVariant::new("active", Arc::new(MockProvider::new("active", "a")), 1.0, 1),
            ModelVariant::new(
                "inactive",
                Arc::new(MockProvider::new("inactive", "b")),
                0.0,
                2,
            ),
        ];
        let server = ModelServer::new(variants, PromotionPolicy::Manual);

        let msgs = vec![ChatMessage::user("test")];
        for _ in 0..50 {
            let r = server.serve(&msgs).await.unwrap();
            assert_eq!(r.variant_name, "active");
        }
    }

    #[tokio::test]
    async fn test_multiple_variants_serve() {
        // Use clearly different weights so deterministic selection hits multiple variants
        let variants = vec![
            ModelVariant::new("v1", Arc::new(MockProvider::new("v1", "r1")), 0.5, 1),
            ModelVariant::new("v2", Arc::new(MockProvider::new("v2", "r2")), 0.3, 2),
            ModelVariant::new("v3", Arc::new(MockProvider::new("v3", "r3")), 0.2, 3),
        ];
        let server = ModelServer::new(variants, PromotionPolicy::Manual);

        let msgs = vec![ChatMessage::user("test")];
        let mut names = std::collections::HashSet::new();
        for _ in 0..10000 {
            let r = server.serve(&msgs).await.unwrap();
            names.insert(r.variant_name);
        }
        // All three variants should have been selected at least once
        assert_eq!(names.len(), 3, "expected all 3 variants, got {:?}", names);
    }

    #[tokio::test]
    async fn test_no_variants_returns_error() {
        let server = ModelServer::new(vec![], PromotionPolicy::Manual);
        let msgs = vec![ChatMessage::user("test")];
        let result = server.serve(&msgs).await;
        assert!(result.is_err());
    }
}
