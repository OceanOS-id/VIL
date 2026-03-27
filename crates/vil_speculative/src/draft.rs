use async_trait::async_trait;
use vil_llm::ChatMessage;

/// Error from draft generation.
#[derive(Debug)]
pub enum DraftError {
    GenerationFailed(String),
}

impl std::fmt::Display for DraftError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GenerationFailed(e) => write!(f, "draft generation failed: {}", e),
        }
    }
}
impl std::error::Error for DraftError {}

/// Trait for a small/fast draft model that proposes candidate tokens.
///
/// The draft provider generates N candidate token strings given the current
/// conversation context. These candidates are then verified by the target model.
#[async_trait]
pub trait DraftProvider: Send + Sync {
    /// Generate `n_tokens` candidate continuations given the conversation so far.
    ///
    /// Returns a Vec of token strings (one per draft position).
    async fn draft(
        &self,
        messages: &[ChatMessage],
        n_tokens: usize,
    ) -> Result<Vec<String>, DraftError>;

    /// Name of the draft model.
    fn model_name(&self) -> &str;
}
