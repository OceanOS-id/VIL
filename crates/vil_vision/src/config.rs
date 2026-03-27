use serde::{Deserialize, Serialize};

/// Configuration for image analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionConfig {
    /// Maximum image dimension (width or height) before downscaling.
    pub max_dimension: u32,
    /// Whether to perform OCR (text extraction from image).
    pub ocr_enabled: bool,
    /// Whether to detect and label objects.
    pub object_detection: bool,
    /// Minimum confidence threshold for detected objects (0.0 to 1.0).
    pub min_confidence: f32,
    /// Model identifier hint.
    pub model: String,
}

impl Default for VisionConfig {
    fn default() -> Self {
        Self {
            max_dimension: 1024,
            ocr_enabled: true,
            object_detection: true,
            min_confidence: 0.5,
            model: "default".to_string(),
        }
    }
}

impl VisionConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn max_dimension(mut self, d: u32) -> Self {
        self.max_dimension = d;
        self
    }

    pub fn ocr_enabled(mut self, enabled: bool) -> Self {
        self.ocr_enabled = enabled;
        self
    }

    pub fn object_detection(mut self, enabled: bool) -> Self {
        self.object_detection = enabled;
        self
    }

    pub fn min_confidence(mut self, c: f32) -> Self {
        self.min_confidence = c.clamp(0.0, 1.0);
        self
    }

    pub fn model(mut self, m: &str) -> Self {
        self.model = m.to_string();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let cfg = VisionConfig::new()
            .max_dimension(512)
            .ocr_enabled(false)
            .object_detection(true)
            .min_confidence(0.7)
            .model("gpt-4-vision");

        assert_eq!(cfg.max_dimension, 512);
        assert!(!cfg.ocr_enabled);
        assert!(cfg.object_detection);
        assert!((cfg.min_confidence - 0.7).abs() < 0.01);
        assert_eq!(cfg.model, "gpt-4-vision");
    }

    #[test]
    fn test_config_defaults() {
        let cfg = VisionConfig::default();
        assert_eq!(cfg.max_dimension, 1024);
        assert!(cfg.ocr_enabled);
        assert!(cfg.object_detection);
    }
}
