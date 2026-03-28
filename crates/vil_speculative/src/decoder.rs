use std::sync::Arc;
use vil_llm::message::LlmError;
use vil_llm::{ChatMessage, LlmProvider};
use vil_log::app_log;

use crate::config::SpeculativeConfig;
use crate::draft::{DraftError, DraftProvider};
use crate::verifier::verify_draft;

/// Result of a full speculative decode run.
#[derive(Debug, Clone)]
pub struct SpeculativeResult {
    /// The final generated content.
    pub content: String,
    /// Total number of tokens proposed by the draft model.
    pub draft_tokens: usize,
    /// Number of draft tokens accepted by the target model.
    pub accepted_tokens: usize,
    /// Acceptance rate (accepted / draft).
    pub acceptance_rate: f32,
    /// Estimated speedup factor.
    pub speedup: f32,
}

/// Error from the speculative decoding process.
#[derive(Debug)]
pub enum SpeculativeError {
    Draft(DraftError),
    Target(LlmError),
}

impl std::fmt::Display for SpeculativeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Draft(e) => write!(f, "draft error: {}", e),
            Self::Target(e) => write!(f, "target error: {}", e),
        }
    }
}
impl std::error::Error for SpeculativeError {}

impl From<DraftError> for SpeculativeError {
    fn from(e: DraftError) -> Self {
        Self::Draft(e)
    }
}

impl From<LlmError> for SpeculativeError {
    fn from(e: LlmError) -> Self {
        Self::Target(e)
    }
}

/// Speculative decoder — uses a fast draft model and a large target model.
///
/// Flow per iteration:
/// 1. Draft model generates N candidate tokens.
/// 2. Target model verifies all candidates in one call.
/// 3. Accept the matching prefix from the draft.
/// 4. If mismatch, use the target model's token at the divergence point.
/// 5. Repeat until done.
pub struct SpeculativeDecoder {
    pub draft: Arc<dyn DraftProvider>,
    pub target: Arc<dyn LlmProvider>,
    pub config: SpeculativeConfig,
}

impl SpeculativeDecoder {
    pub fn new(
        draft: Arc<dyn DraftProvider>,
        target: Arc<dyn LlmProvider>,
        config: SpeculativeConfig,
    ) -> Self {
        Self {
            draft,
            target,
            config,
        }
    }

    /// Run speculative decoding on the given messages.
    pub async fn decode(
        &self,
        messages: &[ChatMessage],
    ) -> Result<SpeculativeResult, SpeculativeError> {
        let mut output = String::new();
        let mut total_draft = 0usize;
        let mut total_accepted = 0usize;
        let mut iterations = 0usize;

        loop {
            if iterations >= self.config.max_iterations {
                app_log!(Debug, "speculative_decode", { event: "max_iterations", max: self.config.max_iterations });
                break;
            }
            if output.len() >= self.config.max_total_tokens {
                app_log!(Debug, "speculative_decode", { event: "max_tokens", max: self.config.max_total_tokens });
                break;
            }
            iterations += 1;

            // Build context with output so far.
            let mut context = messages.to_vec();
            if !output.is_empty() {
                context.push(ChatMessage::assistant(&output));
            }

            // Step 1: Draft generates candidate tokens.
            let draft_tokens = self
                .draft
                .draft(&context, self.config.max_draft_tokens)
                .await?;

            if draft_tokens.is_empty() {
                app_log!(Debug, "speculative_decode", { event: "draft_empty" });
                break;
            }

            let n_draft = draft_tokens.len();
            total_draft += n_draft;

            // Step 2+3: Verify draft against target.
            let verification = verify_draft(&self.target, &context, &draft_tokens).await?;

            if verification.accepted > 0 {
                // Accept matching prefix from draft.
                let accepted_text: String = draft_tokens[..verification.accepted].join("");
                output.push_str(&accepted_text);
                total_accepted += verification.accepted;

                app_log!(Debug, "speculative_decode", { accepted: verification.accepted, drafted: n_draft });
            }

            // Step 4: If not all tokens accepted, use target's correction.
            if verification.accepted < n_draft {
                // Use target's content as the correction token.
                if !verification.target_content.is_empty() {
                    output.push_str(&verification.target_content);
                }
                app_log!(Debug, "speculative_decode", { event: "diverged", position: verification.accepted });
            }

            // If all draft tokens accepted and no more content, we are done.
            if verification.accepted == n_draft && verification.target_content.is_empty() {
                app_log!(Debug, "speculative_decode", { event: "all_accepted_done" });
                break;
            }

            // If the target returned content that signals end, break.
            if verification.target_content.is_empty() && verification.accepted == 0 {
                app_log!(Debug, "speculative_decode", { event: "no_progress_done" });
                break;
            }
        }

        let acceptance_rate = if total_draft > 0 {
            total_accepted as f32 / total_draft as f32
        } else {
            0.0
        };

        // Speedup estimate: each accepted token saves a target call.
        // Base: 1 target call per token. With speculation: 1 target call per (accepted+1) tokens.
        let speedup = if total_draft > 0 && total_accepted > 0 {
            let tokens_generated = total_accepted as f32;
            let target_calls = (total_draft as f32 / self.config.max_draft_tokens as f32).ceil();
            if target_calls > 0.0 {
                tokens_generated / target_calls
            } else {
                1.0
            }
        } else {
            1.0
        };

        app_log!(Info, "speculative_decode_complete", {
            content_len: output.len(),
            total_draft: total_draft,
            total_accepted: total_accepted
        });

        Ok(SpeculativeResult {
            content: output,
            draft_tokens: total_draft,
            accepted_tokens: total_accepted,
            acceptance_rate,
            speedup,
        })
    }
}
