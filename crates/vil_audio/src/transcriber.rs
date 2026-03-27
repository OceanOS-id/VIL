use async_trait::async_trait;

use crate::config::TranscriptConfig;
use crate::result::Transcript;

/// Error type for transcription operations.
#[derive(Debug, Clone)]
pub enum TranscribeError {
    /// Unsupported audio format.
    UnsupportedFormat(String),
    /// The audio data is too short or empty.
    EmptyAudio,
    /// Model not available.
    ModelNotFound(String),
    /// Generic transcription failure.
    TranscriptionFailed(String),
}

impl std::fmt::Display for TranscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TranscribeError::UnsupportedFormat(fmt) => write!(f, "unsupported format: {}", fmt),
            TranscribeError::EmptyAudio => write!(f, "audio data is empty"),
            TranscribeError::ModelNotFound(m) => write!(f, "model not found: {}", m),
            TranscribeError::TranscriptionFailed(e) => write!(f, "transcription failed: {}", e),
        }
    }
}

impl std::error::Error for TranscribeError {}

/// Core trait for audio transcription backends.
#[async_trait]
pub trait Transcriber: Send + Sync {
    /// Transcribe audio bytes into text.
    async fn transcribe(&self, audio: &[u8]) -> Result<Transcript, TranscribeError>;

    /// Transcribe with explicit configuration.
    async fn transcribe_with_config(
        &self,
        audio: &[u8],
        config: &TranscriptConfig,
    ) -> Result<Transcript, TranscribeError>;

    /// Name of this transcriber backend.
    fn name(&self) -> &str;
}

/// A no-op transcriber that returns an error — base implementation — extend for real backends.
pub struct NoopTranscriber;

#[async_trait]
impl Transcriber for NoopTranscriber {
    async fn transcribe(&self, audio: &[u8]) -> Result<Transcript, TranscribeError> {
        if audio.is_empty() {
            return Err(TranscribeError::EmptyAudio);
        }
        Err(TranscribeError::ModelNotFound(
            "no transcription backend configured".into(),
        ))
    }

    async fn transcribe_with_config(
        &self,
        audio: &[u8],
        _config: &TranscriptConfig,
    ) -> Result<Transcript, TranscribeError> {
        self.transcribe(audio).await
    }

    fn name(&self) -> &str {
        "noop"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_noop_transcriber_empty() {
        let t = NoopTranscriber;
        let result = t.transcribe(b"").await;
        assert!(matches!(result, Err(TranscribeError::EmptyAudio)));
    }

    #[tokio::test]
    async fn test_noop_transcriber_no_backend() {
        let t = NoopTranscriber;
        let result = t.transcribe(b"some audio data").await;
        assert!(matches!(result, Err(TranscribeError::ModelNotFound(_))));
    }
}
