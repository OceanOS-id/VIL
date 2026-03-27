use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Result of analyzing an image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageAnalysis {
    /// Human-readable description of the image content.
    pub description: String,
    /// Detected objects in the image.
    pub objects: Vec<DetectedObject>,
    /// Text extracted from the image via OCR, if any.
    pub text_content: Option<String>,
}

/// A detected object within an image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedObject {
    /// Label/class of the detected object.
    pub label: String,
    /// Detection confidence (0.0 to 1.0).
    pub confidence: f32,
    /// Bounding box as [x_min, y_min, x_max, y_max] in normalized coordinates (0.0 to 1.0).
    pub bounding_box: [f32; 4],
}

impl DetectedObject {
    /// Compute bounding box area (normalized, 0.0 to 1.0).
    pub fn area(&self) -> f32 {
        let width = (self.bounding_box[2] - self.bounding_box[0]).max(0.0);
        let height = (self.bounding_box[3] - self.bounding_box[1]).max(0.0);
        width * height
    }
}

/// Error type for image analysis operations.
#[derive(Debug, Clone)]
pub enum VisionError {
    UnsupportedFormat(String),
    EmptyImage,
    AnalysisFailed(String),
}

impl std::fmt::Display for VisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VisionError::UnsupportedFormat(fmt) => write!(f, "unsupported image format: {}", fmt),
            VisionError::EmptyImage => write!(f, "image data is empty"),
            VisionError::AnalysisFailed(e) => write!(f, "analysis failed: {}", e),
        }
    }
}

impl std::error::Error for VisionError {}

/// Core trait for image analysis backends.
#[async_trait]
pub trait ImageAnalyzer: Send + Sync {
    /// Analyze an image and return structured results.
    async fn analyze(&self, image: &[u8]) -> Result<ImageAnalysis, VisionError>;

    /// Name of this analyzer backend.
    fn name(&self) -> &str;
}

/// A no-op analyzer that returns an error — extend with real backend (OpenAI Vision, Tesseract, etc.).
pub struct NoopAnalyzer;

#[async_trait]
impl ImageAnalyzer for NoopAnalyzer {
    async fn analyze(&self, image: &[u8]) -> Result<ImageAnalysis, VisionError> {
        if image.is_empty() {
            return Err(VisionError::EmptyImage);
        }
        Err(VisionError::AnalysisFailed(
            "no analysis backend configured".into(),
        ))
    }

    fn name(&self) -> &str {
        "noop"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detected_object_area() {
        let obj = DetectedObject {
            label: "cat".into(),
            confidence: 0.9,
            bounding_box: [0.1, 0.2, 0.5, 0.8],
        };
        let area = obj.area();
        // (0.5 - 0.1) * (0.8 - 0.2) = 0.4 * 0.6 = 0.24
        assert!((area - 0.24).abs() < 0.001);
    }

    #[test]
    fn test_analysis_types() {
        let analysis = ImageAnalysis {
            description: "A cat sitting on a mat".into(),
            objects: vec![DetectedObject {
                label: "cat".into(),
                confidence: 0.95,
                bounding_box: [0.1, 0.1, 0.9, 0.9],
            }],
            text_content: None,
        };
        assert_eq!(analysis.objects.len(), 1);
        assert_eq!(analysis.objects[0].label, "cat");
    }

    #[tokio::test]
    async fn test_noop_analyzer_empty() {
        let a = NoopAnalyzer;
        let result = a.analyze(b"").await;
        assert!(matches!(result, Err(VisionError::EmptyImage)));
    }

    #[tokio::test]
    async fn test_noop_analyzer_no_backend() {
        let a = NoopAnalyzer;
        let result = a.analyze(b"fake image data").await;
        assert!(matches!(result, Err(VisionError::AnalysisFailed(_))));
    }
}
