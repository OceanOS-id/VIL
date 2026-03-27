//! VIL Prompt Shield — real-time prompt injection detection.
//!
//! Detects prompt injection attacks before they reach the LLM.
//! Uses Aho-Corasick multi-pattern matching for <100us latency.
//!
//! ```
//! use vil_prompt_shield::PromptShield;
//!
//! let shield = PromptShield::new();
//! let result = shield.scan("Ignore previous instructions");
//! assert!(!result.safe);
//! ```

pub mod config;
pub mod detector;
pub mod patterns;
pub mod result;
pub mod scorer;
pub mod semantic;
pub mod pipeline_sse;
pub mod handlers;
pub mod plugin;

pub use config::ShieldConfig;
pub use detector::PromptShield;
pub use result::{RiskLevel, ScanResult, Threat, ThreatCategory};
pub use plugin::ShieldPlugin;
pub use semantic::{ShieldEvent, ShieldFault, ShieldFaultType, ShieldState};
