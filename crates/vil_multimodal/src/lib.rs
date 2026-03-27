// ── vil_multimodal ── N05: Multimodal Fusion ──────────────────────
//!
//! Cross-modality embedding fusion and search.
//! Supports weighted average, concatenation, and brute-force
//! cosine-similarity search across Text, Image, Audio, Video.

pub mod fusion;
pub mod modality;
pub mod search;

pub use fusion::{concatenate, weighted_average, FusionEngine, FusionError};
pub use modality::{Modality, MultimodalEmbedding};
pub use search::MultimodalSearch;

// VIL integration layer
pub mod vil_semantic;
pub mod pipeline_sse;
pub mod handlers;
pub mod plugin;

pub use plugin::MultimodalPlugin;
pub use vil_semantic::{MultimodalEvent, MultimodalFault, MultimodalState};
