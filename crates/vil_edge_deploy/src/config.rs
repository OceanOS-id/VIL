// =============================================================================
// vil_edge_deploy::config — EdgeConfig
// =============================================================================

use serde::{Deserialize, Serialize};

use crate::{
    profile::{EdgeProfile, ProfilePreset},
    targets::EdgeTarget,
};

/// Scheduler behaviour for the edge runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SchedulerMode {
    /// All processing pinned to a single CPU core.
    #[default]
    SingleCore,
    /// Work-stealing across all available CPU cores.
    MultiCore,
}

impl std::fmt::Display for SchedulerMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchedulerMode::SingleCore => write!(f, "single_core"),
            SchedulerMode::MultiCore  => write!(f, "multi_core"),
        }
    }
}

/// Complete edge deployment configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeConfig {
    /// Cross-compile target architecture.
    pub target: EdgeTarget,
    /// Resource profile preset.
    pub profile: EdgeProfile,
    /// Shared memory size in kilobytes. Default: 4096 (4 MB).
    pub shm_size_kb: u32,
    /// Maximum concurrent VIL processes. Default: 16.
    pub max_processes: u16,
    /// Scheduler mode (single-core vs multi-core).
    pub scheduler_mode: SchedulerMode,
    /// Offline data buffer size in kilobytes. Default: 512.
    pub offline_buffer_kb: u32,
}

impl Default for EdgeConfig {
    fn default() -> Self {
        let preset = EdgeProfile::Minimal.preset();
        Self {
            target:            EdgeTarget::X86_64Linux,
            profile:           EdgeProfile::Minimal,
            shm_size_kb:       preset.shm_size_kb,
            max_processes:     preset.max_processes,
            scheduler_mode:    preset.scheduler_mode,
            offline_buffer_kb: preset.offline_buffer_kb,
        }
    }
}

impl EdgeConfig {
    /// Create config from a profile preset applied to a given target.
    pub fn from_profile(target: EdgeTarget, profile: EdgeProfile) -> Self {
        let preset: ProfilePreset = profile.preset();
        Self {
            target,
            profile,
            shm_size_kb:       preset.shm_size_kb,
            max_processes:     preset.max_processes,
            scheduler_mode:    preset.scheduler_mode,
            offline_buffer_kb: preset.offline_buffer_kb,
        }
    }

    /// Deserialize from a YAML string.
    pub fn from_yaml(yaml: &str) -> Result<Self, crate::error::EdgeFault> {
        serde_yaml::from_str(yaml).map_err(|_| crate::error::EdgeFault::ConfigParseFailed)
    }

    /// Serialize to a YAML string.
    pub fn to_yaml(&self) -> Result<String, crate::error::EdgeFault> {
        serde_yaml::to_string(self).map_err(|_| crate::error::EdgeFault::SerializeFailed)
    }

    /// Load from a YAML file on disk.
    pub fn from_file(path: &std::path::Path) -> Result<Self, crate::error::EdgeFault> {
        let content = std::fs::read_to_string(path)
            .map_err(|_| crate::error::EdgeFault::ConfigReadFailed)?;
        Self::from_yaml(&content)
    }

    /// Total shared memory in megabytes (rounded).
    pub fn shm_size_mb(&self) -> u32 {
        self.shm_size_kb / 1024
    }

    /// Validate the configuration.
    pub fn validate(&self) -> bool {
        self.shm_size_kb > 0 && self.max_processes > 0
    }
}
