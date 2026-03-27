// =============================================================================
// VIL Engine — Unified Core Binary Interface
// =============================================================================
// This crate aggregates all core physics modules (rt, shm, queue, registry)
// into a single static library. In a production environment, this crate
// would be distributed as a pre-compiled binary (.a / .lib).
// =============================================================================

pub use vil_rt as rt;
pub use vil_shm as shm;
pub use vil_queue as queue;
pub use vil_registry as registry;
pub use vil_types as types;
pub use vil_obs as obs;

/// Entry point to initialize the engine world from the binary.
pub fn create_runtime_world() -> std::sync::Arc<vil_rt::VastarRuntimeWorld> {
    std::sync::Arc::new(vil_rt::VastarRuntimeWorld::new())
}

// Ensure the symbols are exported for static linking
#[no_mangle]
pub extern "C" fn vil_engine_version() -> *const i8 {
    "0.1.0\0".as_ptr() as *const i8
}
