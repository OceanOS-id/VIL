//! Resource constraints for edge devices.

use serde::{Deserialize, Serialize};

/// Target architecture for edge deployment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetArch {
    X86_64,
    Aarch64,
    Riscv64,
    Wasm32,
}

impl std::fmt::Display for TargetArch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::X86_64 => write!(f, "x86_64"),
            Self::Aarch64 => write!(f, "aarch64"),
            Self::Riscv64 => write!(f, "riscv64"),
            Self::Wasm32 => write!(f, "wasm32"),
        }
    }
}

/// Configuration for edge runtime resource constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeConfig {
    /// Maximum available memory in MB.
    pub max_memory_mb: u64,
    /// Maximum allowed model size in MB.
    pub max_model_size_mb: u64,
    /// Target architecture.
    pub target_arch: TargetArch,
}

impl Default for EdgeConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: 512,
            max_model_size_mb: 256,
            target_arch: TargetArch::Aarch64,
        }
    }
}
