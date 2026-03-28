// =============================================================================
// vil_log — VIL Semantic Log System
// =============================================================================
//
// Zero-copy, non-blocking, structured logging with pluggable drains.
//
// Architecture:
//   - `types`   — VilLogHeader, LogSlot, LogLevel, LogCategory, payload structs
//   - `emit`    — Global SPSC ring, macros (app_log!, access_log!, …), tracing layer
//   - `drain`   — Pluggable drains: stdout, file, null, multi
//   - `config`  — LogConfig struct
//   - `runtime` — init_logging, drain task
//   - `dict`    — Hash → string reverse lookup registry
//
// Quick start:
// ```rust,ignore
// use vil_log::{init_logging, LogConfig, StdoutDrain};
// use vil_log::app_log;
//
// #[tokio::main]
// async fn main() {
//     let _task = init_logging(LogConfig::default(), StdoutDrain::pretty());
//     app_log!(Info, "app.start", { version: "0.1.0" });
// }
// ```
// =============================================================================

pub mod config;
pub mod dict;
pub mod drain;
pub mod emit;
pub mod resolve;
pub mod runtime;
pub mod types;

// =============================================================================
// Flat re-exports for ergonomic use
// =============================================================================

pub use config::LogConfig;

pub use drain::{
    FallbackDrain, FileDrain, LogDrain, MultiDrain, NullDrain, RotationStrategy, StdoutDrain,
    StdoutFormat,
};

pub use emit::{global_ring, init_ring, try_global_ring, LogRing, VilTracingLayer};

pub use runtime::{init_logging, init_logging_with_tracing, spawn_drain_task};

pub use types::{
    AccessPayload, AiPayload, AppPayload, DbPayload, LogCategory, LogLevel, LogSlot, MqPayload,
    SecurityPayload, SystemPayload, VilLogHeader,
};
