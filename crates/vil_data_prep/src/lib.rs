// ── vil_data_prep ── N01: Fine-Tuning Data Pipeline ───────────────
//!
//! Chain-based data pipeline for LLM fine-tuning data preparation.
//! Supports deduplication (exact + fuzzy), quality filtering, and
//! multi-format output (JSONL, Alpaca, ShareGPT, ChatML).

pub mod dedup;
pub mod filter;
pub mod formatter;
pub mod pipeline;

pub use filter::QualityFilter;
pub use formatter::{OutputFormat, TrainingRecord};
pub use pipeline::{DataPipeline, PipelineResult, PipelineStep};

// VIL integration layer
pub mod handlers;
pub mod pipeline_sse;
pub mod plugin;
pub mod vil_semantic;

pub use plugin::DataPrepPlugin;
pub use vil_semantic::{DataPrepEvent, DataPrepFault, DataPrepState};
