//! Configuration types for the AI pipeline compiler.

use serde::{Deserialize, Serialize};

/// Top-level compiler configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerConfig {
    /// Whether to fuse consecutive Transform nodes.
    pub fuse_transforms: bool,
    /// Whether to eliminate redundant Cache nodes.
    pub eliminate_redundant_caches: bool,
    /// Maximum parallelism (0 = unlimited).
    pub max_parallelism: usize,
    /// Enable tracing instrumentation in compiled plan.
    pub enable_tracing: bool,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            fuse_transforms: true,
            eliminate_redundant_caches: true,
            max_parallelism: 0,
            enable_tracing: true,
        }
    }
}

impl CompilerConfig {
    /// Create a config with all optimizations disabled.
    pub fn no_optimizations() -> Self {
        Self {
            fuse_transforms: false,
            eliminate_redundant_caches: false,
            max_parallelism: 0,
            enable_tracing: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = CompilerConfig::default();
        assert!(cfg.fuse_transforms);
        assert!(cfg.eliminate_redundant_caches);
        assert_eq!(cfg.max_parallelism, 0);
    }

    #[test]
    fn test_serde_roundtrip() {
        let cfg = CompilerConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let back: CompilerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.fuse_transforms, cfg.fuse_transforms);
    }
}
