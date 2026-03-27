use async_trait::async_trait;
use std::fmt;

/// Errors that can occur during embedding operations.
#[derive(Debug, Clone)]
pub enum EmbedError {
    /// The HTTP request or API call failed.
    RequestFailed(String),
    /// The returned embedding dimension doesn't match what was expected.
    DimensionMismatch { expected: usize, got: usize },
    /// The API returned an empty result set.
    EmptyResult,
    /// The provider is rate-limited; caller should retry later.
    RateLimited,
}

impl fmt::Display for EmbedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmbedError::RequestFailed(msg) => write!(f, "request failed: {msg}"),
            EmbedError::DimensionMismatch { expected, got } => {
                write!(f, "dimension mismatch: expected {expected}, got {got}")
            }
            EmbedError::EmptyResult => write!(f, "empty result"),
            EmbedError::RateLimited => write!(f, "rate limited"),
        }
    }
}

impl std::error::Error for EmbedError {}

/// Trait for embedding providers (OpenAI, local ONNX, etc.).
///
/// Implementors must be `Send + Sync` so they can be shared across
/// async tasks and wrapped in `Arc`.
#[async_trait]
pub trait EmbedProvider: Send + Sync {
    /// Embed a batch of texts, returning one vector per input text.
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError>;

    /// Embed a single text. Default implementation delegates to `embed_batch`.
    async fn embed_one(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        let results = self.embed_batch(&[text.to_string()]).await?;
        results.into_iter().next().ok_or(EmbedError::EmptyResult)
    }

    /// The dimensionality of vectors produced by this provider.
    fn dimension(&self) -> usize;

    /// Human-readable model name (e.g. "text-embedding-3-small").
    fn model_name(&self) -> &str;

    /// Maximum number of texts the provider can handle in one API call.
    fn max_batch_size(&self) -> usize {
        100
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embed_error_display() {
        let err = EmbedError::RequestFailed("timeout".into());
        assert_eq!(format!("{err}"), "request failed: timeout");

        let err = EmbedError::DimensionMismatch {
            expected: 1536,
            got: 768,
        };
        assert!(format!("{err}").contains("1536"));

        let err = EmbedError::EmptyResult;
        assert_eq!(format!("{err}"), "empty result");

        let err = EmbedError::RateLimited;
        assert_eq!(format!("{err}"), "rate limited");
    }
}
