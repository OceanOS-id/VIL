// ── vil_rlhf_data ── N03: RLHF/DPO Pipeline ──────────────────────
//!
//! Preference-pair dataset management for RLHF and DPO training.
//! Load, edit, export to DPO/RLHF training formats.

pub mod dataset;
pub mod formatter;
pub mod preference;

pub use dataset::{DatasetStats, PreferenceDataset};
pub use preference::PreferencePair;

// VIL integration layer
pub mod vil_semantic;
pub mod pipeline_sse;
pub mod handlers;
pub mod plugin;

pub use plugin::RlhfPlugin;
pub use vil_semantic::{RlhfEvent, RlhfFault, RlhfState};
