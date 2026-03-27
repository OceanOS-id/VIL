// =============================================================================
// vil_edge_deploy — VIL Edge Deployment Profiles
// =============================================================================
//
// Configuration and build helpers for edge/IoT deployment targeting
// ARM (aarch64, armv7) and RISC-V (riscv64gc) platforms.
//
// Architecture:
//   - `config`   — EdgeConfig struct (target, profile, shm, processes, scheduler)
//   - `profile`  — EdgeProfile enum (Minimal, Standard, Full) with presets
//   - `targets`  — EdgeTarget enum with rustc_target_triple() / cargo_build_args()
//   - `error`    — EdgeFault (plain enum, register_str-hashed codes)
//   - `process`  — create() builds an EdgeConfig ready for use
//
// Quick start:
// ```rust,ignore
// use vil_edge_deploy::{process, EdgeTarget, EdgeProfile};
//
// let config = process::create(EdgeTarget::Aarch64Linux, EdgeProfile::Standard)?;
// let args   = config.target.cargo_build_args();
// // → ["--target", "aarch64-unknown-linux-gnu"]
// ```
// =============================================================================

pub mod config;
pub mod error;
pub mod process;
pub mod profile;
pub mod state;
pub mod targets;

pub use config::{EdgeConfig, SchedulerMode};
pub use error::EdgeFault;
pub use process::create;
pub use profile::EdgeProfile;
pub use state::EdgeDeployState;
pub use targets::EdgeTarget;
