//! VIL Speculative Decoding Proxy.
//!
//! Implements speculative decoding: a small/fast draft model proposes candidate
//! tokens, then a large target model verifies them in a single forward pass.
//! When draft and target agree, multiple tokens are accepted at once, yielding
//! 2-3x faster generation without quality loss.
//!
//! # Architecture
//!
//! ```text
//! User Prompt
//!     |
//!     v
//! ┌─────────────────┐     ┌──────────────┐
//! │  DraftProvider   │────>│   Verifier   │
//! │  (small model)   │     │ (target LLM) │
//! │  N tokens fast   │     │ 1-call verify│
//! └─────────────────┘     └──────┬───────┘
//!                                │
//!              accept prefix + correct divergence
//!                                │
//!                                v
//!                       SpeculativeResult
//! ```

pub mod config;
pub mod draft;
pub mod verifier;
pub mod decoder;

pub use config::SpeculativeConfig;
pub use draft::{DraftProvider, DraftError};
pub use verifier::{verify_draft, VerificationResult};
pub use decoder::{SpeculativeDecoder, SpeculativeResult, SpeculativeError};

pub mod semantic;
pub mod pipeline_sse;
pub mod handlers;
pub mod plugin;

pub use plugin::SpeculativePlugin;
pub use semantic::{SpeculativeEvent, SpeculativeFault, SpeculativeState};

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use vil_llm::message::{ChatMessage, ChatResponse, LlmError};
    use vil_llm::provider::LlmProvider;

    // ───────────────────────── Mock DraftProvider ─────────────────────────

    /// Mock draft that returns pre-configured tokens.
    struct MockDraft {
        /// Tokens to return for each call to draft().
        tokens: Vec<Vec<String>>,
        call_count: AtomicUsize,
    }

    impl MockDraft {
        fn new(tokens: Vec<Vec<String>>) -> Self {
            Self {
                tokens,
                call_count: AtomicUsize::new(0),
            }
        }

}

    #[async_trait]
    impl DraftProvider for MockDraft {
        async fn draft(
            &self,
            _messages: &[ChatMessage],
            n_tokens: usize,
        ) -> Result<Vec<String>, DraftError> {
            let idx = self.call_count.fetch_add(1, Ordering::SeqCst);
            if idx < self.tokens.len() {
                let t = &self.tokens[idx];
                Ok(t[..t.len().min(n_tokens)].to_vec())
            } else {
                // No more tokens to draft.
                Ok(vec![])
            }
        }

        fn model_name(&self) -> &str {
            "mock-draft"
        }
    }

    // ───────────────────────── Mock LlmProvider (Target) ─────────────────

    /// Mock target that returns pre-configured responses.
    struct MockTarget {
        /// Responses to return for each call to chat().
        responses: Vec<String>,
        call_count: AtomicUsize,
    }

    impl MockTarget {
        fn new(responses: Vec<String>) -> Self {
            Self {
                responses,
                call_count: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl LlmProvider for MockTarget {
        async fn chat(&self, _messages: &[ChatMessage]) -> Result<ChatResponse, LlmError> {
            let idx = self.call_count.fetch_add(1, Ordering::SeqCst);
            let content = if idx < self.responses.len() {
                self.responses[idx].clone()
            } else {
                String::new()
            };
            Ok(ChatResponse {
                content,
                model: "mock-target".to_string(),
                tool_calls: None,
                usage: None,
                finish_reason: Some("stop".to_string()),
            })
        }

        fn model(&self) -> &str {
            "mock-target"
        }

        fn provider_name(&self) -> &str {
            "mock"
        }
    }

    // ───────────────────────── Tests ─────────────────────────

    #[tokio::test]
    async fn test_full_decode_with_mock() {
        // Draft proposes "Hello world", target confirms by returning matching prefix.
        let draft = Arc::new(MockDraft::new(vec![
            vec!["Hello".into(), " world".into()],
        ]));
        // Target sees draft as assistant message, returns empty (confirming).
        let target = Arc::new(MockTarget::new(vec!["".into()]));

        let decoder = SpeculativeDecoder::new(
            draft,
            target,
            SpeculativeConfig::new().max_draft_tokens(2).max_iterations(2),
        );

        let result = decoder
            .decode(&[ChatMessage::user("Say hello")])
            .await
            .unwrap();

        // The draft tokens were "Hello" + " world". Target returned empty,
        // so no tokens match (count_matching_prefix against "").
        // The decoder should finish with whatever was produced.
        assert!(result.draft_tokens > 0);
    }

    #[tokio::test]
    async fn test_high_acceptance_rate() {
        // Draft: ["The", " cat"], Target confirms with "The cat" prefix.
        let draft = Arc::new(MockDraft::new(vec![
            vec!["The".into(), " cat".into()],
        ]));
        let target = Arc::new(MockTarget::new(vec![
            "The cat".into(), // matches both draft tokens
        ]));

        let decoder = SpeculativeDecoder::new(
            draft,
            target,
            SpeculativeConfig::new().max_draft_tokens(2).max_iterations(2),
        );

        let result = decoder
            .decode(&[ChatMessage::user("Complete the sentence")])
            .await
            .unwrap();

        assert_eq!(result.accepted_tokens, 2);
        assert_eq!(result.draft_tokens, 2);
        assert!((result.acceptance_rate - 1.0).abs() < f32::EPSILON);
        assert!(result.content.contains("The"));
        assert!(result.content.contains("cat"));
    }

    #[tokio::test]
    async fn test_zero_acceptance_all_rejected() {
        // Draft proposes tokens that don't match target at all.
        let draft = Arc::new(MockDraft::new(vec![
            vec!["foo".into(), "bar".into()],
        ]));
        let target = Arc::new(MockTarget::new(vec![
            "completely different".into(),
        ]));

        let decoder = SpeculativeDecoder::new(
            draft,
            target,
            SpeculativeConfig::new().max_draft_tokens(2).max_iterations(2),
        );

        let result = decoder
            .decode(&[ChatMessage::user("test")])
            .await
            .unwrap();

        assert_eq!(result.accepted_tokens, 0);
        assert_eq!(result.draft_tokens, 2);
        assert!((result.acceptance_rate - 0.0).abs() < f32::EPSILON);
        // Target's correction should appear in output.
        assert!(result.content.contains("completely different"));
    }

    #[tokio::test]
    async fn test_empty_input() {
        let draft = Arc::new(MockDraft::new(vec![
            vec!["Hello".into()],
        ]));
        let target = Arc::new(MockTarget::new(vec!["Hello".into()]));

        let decoder = SpeculativeDecoder::new(
            draft,
            target,
            SpeculativeConfig::new().max_draft_tokens(1).max_iterations(2),
        );

        // Empty messages array.
        let result = decoder.decode(&[]).await.unwrap();
        assert!(!result.content.is_empty());
    }

    #[tokio::test]
    async fn test_config_builder() {
        let config = SpeculativeConfig::new()
            .max_draft_tokens(8)
            .max_total_tokens(512)
            .max_iterations(50);

        assert_eq!(config.max_draft_tokens, 8);
        assert_eq!(config.max_total_tokens, 512);
        assert_eq!(config.max_iterations, 50);
    }

    #[tokio::test]
    async fn test_config_defaults() {
        let config = SpeculativeConfig::default();
        assert_eq!(config.max_draft_tokens, 5);
        assert_eq!(config.max_total_tokens, 256);
        assert_eq!(config.max_iterations, 100);
    }

    #[tokio::test]
    async fn test_partial_acceptance() {
        // Draft: ["The", " quick", " fox"], target matches first two only.
        let draft = Arc::new(MockDraft::new(vec![
            vec!["The".into(), " quick".into(), " fox".into()],
        ]));
        let target = Arc::new(MockTarget::new(vec![
            "The quick brown".into(), // matches "The" and " quick" but not " fox"
        ]));

        let decoder = SpeculativeDecoder::new(
            draft,
            target,
            SpeculativeConfig::new().max_draft_tokens(3).max_iterations(2),
        );

        let result = decoder
            .decode(&[ChatMessage::user("test")])
            .await
            .unwrap();

        assert_eq!(result.accepted_tokens, 2);
        assert_eq!(result.draft_tokens, 3);
        // Should contain accepted prefix + target correction.
        assert!(result.content.contains("The quick"));
    }

    #[tokio::test]
    async fn test_multi_iteration_decode() {
        // Two rounds of drafting.
        let draft = Arc::new(MockDraft::new(vec![
            vec!["Hello".into(), " ".into()],
            vec!["world".into(), "!".into()],
        ]));
        let target = Arc::new(MockTarget::new(vec![
            "Hello ".into(),  // first round: full match
            "world!".into(),  // second round: full match
        ]));

        let decoder = SpeculativeDecoder::new(
            draft,
            target,
            SpeculativeConfig::new().max_draft_tokens(2).max_iterations(5),
        );

        let result = decoder
            .decode(&[ChatMessage::user("greet")])
            .await
            .unwrap();

        assert_eq!(result.accepted_tokens, 4);
        assert_eq!(result.draft_tokens, 4);
        assert!((result.acceptance_rate - 1.0).abs() < f32::EPSILON);
    }
}
