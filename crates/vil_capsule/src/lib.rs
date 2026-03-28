// =============================================================================
// crates/vil_capsule/src/lib.rs — VIL WASM Capsule Runtime
// =============================================================================
// Provides a sandboxed WASM runtime host for executing third-party code
// within trust zone boundaries.
//
// Feature gate: enable the `wasm` feature to activate the wasmtime runtime.
// =============================================================================

pub mod host;
pub mod pool;
pub mod runner;

#[cfg(feature = "wasi")]
pub mod wasi_host;

pub use host::{CapsuleConfig, CapsuleHost, CapsuleInput, CapsuleOutput};
pub use pool::{WasmFaaSConfig, WasmFaaSRegistry, WasmPool};
pub use runner::CapsuleRunner;

#[cfg(feature = "wasi")]
pub use wasi_host::{WasiCapabilities, WasiCapsuleHost};

/// Errors that may occur during the capsule lifecycle.
#[derive(Debug)]
pub enum CapsuleError {
    /// Failed to load the WASM file.
    LoadFailed(String),
    /// Failed to compile the WASM module.
    CompileFailed(String),
    /// Failed to instantiate the WASM module.
    InstantiateFailed(String),
    /// Failed to call a function in the WASM module.
    ExecutionFailed(String),
    /// WASM runtime unavailable (`wasm` feature not enabled).
    WasmFeatureNotEnabled,
}

impl std::fmt::Display for CapsuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LoadFailed(msg) => write!(f, "capsule load failed: {}", msg),
            Self::CompileFailed(msg) => write!(f, "capsule compile failed: {}", msg),
            Self::InstantiateFailed(msg) => write!(f, "capsule instantiate failed: {}", msg),
            Self::ExecutionFailed(msg) => write!(f, "capsule execution failed: {}", msg),
            Self::WasmFeatureNotEnabled => write!(
                f,
                "wasm feature not enabled; recompile with --features wasm"
            ),
        }
    }
}

impl std::error::Error for CapsuleError {}
