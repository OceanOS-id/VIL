//! Guardrails configuration.

use serde::{Deserialize, Serialize};

/// Configuration for the guardrails engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardrailsConfig {
    /// Enable PII detection.
    pub pii_enabled: bool,
    /// Enable toxicity checking.
    pub toxicity_enabled: bool,
    /// Enable custom rule engine.
    pub rules_enabled: bool,
    /// Toxicity score threshold (0.0 - 1.0). Above this, text fails the check.
    pub toxicity_threshold: f32,
}

impl Default for GuardrailsConfig {
    fn default() -> Self {
        Self {
            pii_enabled: true,
            toxicity_enabled: true,
            rules_enabled: true,
            toxicity_threshold: 0.5,
        }
    }
}

/// Builder for GuardrailsConfig.
pub struct GuardrailsConfigBuilder {
    config: GuardrailsConfig,
}

impl GuardrailsConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: GuardrailsConfig::default(),
        }
    }

    pub fn pii_enabled(mut self, enabled: bool) -> Self {
        self.config.pii_enabled = enabled;
        self
    }

    pub fn toxicity_enabled(mut self, enabled: bool) -> Self {
        self.config.toxicity_enabled = enabled;
        self
    }

    pub fn rules_enabled(mut self, enabled: bool) -> Self {
        self.config.rules_enabled = enabled;
        self
    }

    pub fn toxicity_threshold(mut self, threshold: f32) -> Self {
        self.config.toxicity_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    pub fn build(self) -> GuardrailsConfig {
        self.config
    }
}

impl Default for GuardrailsConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
