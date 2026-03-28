// ── vil_synthetic ── N02: Synthetic Data Generator ────────────────
//!
//! Template-based synthetic data generation with quality checking.
//! Expands seed examples into larger fine-tuning datasets.

pub mod generator;
pub mod quality;
pub mod template;

pub use generator::{SeedExample, SyntheticExample, SyntheticGenerator};
pub use quality::QualityChecker;
pub use template::GenerationTemplate;

// VIL integration layer
pub mod handlers;
pub mod pipeline_sse;
pub mod plugin;
pub mod vil_semantic;

pub use plugin::SyntheticPlugin;
pub use vil_semantic::{SyntheticEvent, SyntheticFault, SyntheticState};
