// =============================================================================
// vil_edge_deploy::profile — EdgeProfile presets
// =============================================================================

use serde::{Deserialize, Serialize};

use crate::config::SchedulerMode;

/// Deployment profile controlling resource limits and scheduler behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EdgeProfile {
    /// Minimal: 4 MB SHM, 16 processes, single-core scheduler.
    /// Suitable for microcontrollers and very constrained IoT devices.
    Minimal,
    /// Standard: 64 MB SHM, 64 processes, multi-core scheduler.
    /// Suitable for Raspberry Pi, industrial gateways.
    #[default]
    Standard,
    /// Full: production-equivalent limits.
    /// Suitable for server-class edge hardware.
    Full,
}

impl std::fmt::Display for EdgeProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EdgeProfile::Minimal  => write!(f, "Minimal"),
            EdgeProfile::Standard => write!(f, "Standard"),
            EdgeProfile::Full     => write!(f, "Full"),
        }
    }
}

/// Preset values bundled with each profile.
#[derive(Debug, Clone, Copy)]
pub struct ProfilePreset {
    pub shm_size_kb:       u32,
    pub max_processes:     u16,
    pub scheduler_mode:    SchedulerMode,
    pub offline_buffer_kb: u32,
}

impl EdgeProfile {
    /// Return the default resource preset for this profile.
    pub fn preset(self) -> ProfilePreset {
        match self {
            EdgeProfile::Minimal => ProfilePreset {
                shm_size_kb:       4_096,   // 4 MB
                max_processes:     16,
                scheduler_mode:    SchedulerMode::SingleCore,
                offline_buffer_kb: 512,
            },
            EdgeProfile::Standard => ProfilePreset {
                shm_size_kb:       65_536,  // 64 MB
                max_processes:     64,
                scheduler_mode:    SchedulerMode::MultiCore,
                offline_buffer_kb: 8_192,
            },
            EdgeProfile::Full => ProfilePreset {
                shm_size_kb:       262_144, // 256 MB
                max_processes:     256,
                scheduler_mode:    SchedulerMode::MultiCore,
                offline_buffer_kb: 32_768,
            },
        }
    }
}
