// =============================================================================
// vil_rt — Runtime Facade
// =============================================================================
// Provides the public API for the VIL runtime.
// This is the only crate that application developers interact with
// (besides generated code from the compiler).
//
// Main lifecycle:
//   1. VastarRuntimeWorld::new()
//   2. world.register_process(spec) → ProcessHandle
//   3. world.connect(from_port, to_port)
//   4. world.loan_uninit::<T>(port) → Loaned<T>
//   5. loan.write(value) → Loaned<T> (initialized)
//   6. world.publish(owner, port, loan) → Published<T>
//   7. world.recv::<T>(port) → LoanedRead<T>
//   8. world.shutdown_process(pid) / world.crash_process(pid)
//   9. world.metrics_snapshot()
//
// Modules:
//   error.rs      — RtError enum
//   metrics.rs    — RuntimeMetrics snapshot
//   handle.rs     — ProcessHandle, RegisteredPort
//   world.rs      — VastarRuntimeWorld (main API surface)
//   supervisor.rs — Supervisor, CleanupReport (crash cleanup orchestration)
//
// TASK LIST:
// [x] RtError — error types
// [x] RuntimeMetrics — metrics snapshot
// [x] ProcessHandle — per-process handle
// [x] VastarRuntimeWorld — full lifecycle API
// [x] Supervisor — crash cleanup orchestration
// [x] CleanupReport — cleanup audit trail
// [x] Unit tests
// [ ] TODO(future): async recv with waker
// [ ] TODO(future): backpressure enforcement
// [ ] TODO(future): timeout enforcement
// [ ] TODO(future): periodic liveness probe
// =============================================================================

pub mod error;
pub mod metrics;
pub mod handle;
pub mod world;
pub mod supervisor;
pub mod session;
pub mod clock;

pub use error::RtError;
pub use metrics::RuntimeMetrics;
pub use handle::{ProcessHandle, RegisteredPort};
pub use world::VastarRuntimeWorld;
pub use supervisor::{Supervisor, CleanupReport};
pub use session::{SessionRegistry, SessionEntry, PendingSlot, SessionConfig};

// Re-export registry types that users may need for inspection
pub use vil_registry::{SampleRecord, ProcessRecord};

// Re-export observability types
pub use vil_obs::{ObservabilityHub, TraceEvent, RuntimeCounters, LatencyTracker, LatencySnapshot};

// Re-export vil_shm for ShmToken resolve_payload
pub use vil_shm;
