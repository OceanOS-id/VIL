// =============================================================================
// vil_log::emit — Log emission subsystem
// =============================================================================

pub mod macros;
pub mod ring;
pub mod tracing_layer;

pub use ring::{
    global_ring, global_striped, init_ring, try_global_ring, try_global_striped, LogRing,
    StripedRing,
};
pub use tracing_layer::VilTracingLayer;
