use serde::{Deserialize, Serialize};

/// Configuration for audio transcription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptConfig {
    /// Language hint (e.g., "en", "id", "auto").
    pub language: String,
    /// Model size hint (e.g., "tiny", "base", "small", "medium", "large").
    pub model_size: String,
    /// Whether to include word-level timestamps.
    pub timestamps: bool,
    /// Maximum audio duration to process in milliseconds (0 = unlimited).
    pub max_duration_ms: u64,
}

impl Default for TranscriptConfig {
    fn default() -> Self {
        Self {
            language: "auto".to_string(),
            model_size: "base".to_string(),
            timestamps: true,
            max_duration_ms: 0,
        }
    }
}

impl TranscriptConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn language(mut self, lang: &str) -> Self {
        self.language = lang.to_string();
        self
    }

    pub fn model_size(mut self, size: &str) -> Self {
        self.model_size = size.to_string();
        self
    }

    pub fn timestamps(mut self, enabled: bool) -> Self {
        self.timestamps = enabled;
        self
    }

    pub fn max_duration_ms(mut self, ms: u64) -> Self {
        self.max_duration_ms = ms;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let cfg = TranscriptConfig::new()
            .language("en")
            .model_size("large")
            .timestamps(false)
            .max_duration_ms(60_000);

        assert_eq!(cfg.language, "en");
        assert_eq!(cfg.model_size, "large");
        assert!(!cfg.timestamps);
        assert_eq!(cfg.max_duration_ms, 60_000);
    }

    #[test]
    fn test_config_defaults() {
        let cfg = TranscriptConfig::default();
        assert_eq!(cfg.language, "auto");
        assert_eq!(cfg.model_size, "base");
        assert!(cfg.timestamps);
        assert_eq!(cfg.max_duration_ms, 0);
    }
}
