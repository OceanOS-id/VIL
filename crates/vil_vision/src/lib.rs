//! # VIL Image Understanding (I04)
//!
//! Infrastructure and traits for image analysis, object detection, OCR,
//! and image embedding. This crate provides trait definitions and format
//! detection. Actual vision backends plug in via the `ImageAnalyzer` and
//! `ImageEmbedder` traits.
//!
//! ## Quick Start
//!
//! ```rust
//! use vil_vision::{ImageFormat, detect_format, VisionConfig};
//!
//! let png_header = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
//! assert_eq!(detect_format(&png_header), ImageFormat::Png);
//!
//! let config = VisionConfig::new().ocr_enabled(true).min_confidence(0.7);
//! ```

pub mod analyzer;
pub mod config;
pub mod embedding;
pub mod format;
pub mod handlers;
pub mod pipeline_sse;
pub mod plugin;
pub mod semantic;

pub use analyzer::{DetectedObject, ImageAnalysis, ImageAnalyzer, NoopAnalyzer, VisionError};
pub use config::VisionConfig;
pub use embedding::{ImageEmbedder, NoopEmbedder};
pub use format::{detect_format, ImageFormat};
pub use plugin::VisionPlugin;
pub use semantic::{VisionEvent, VisionFault, VisionFaultType, VisionState};
