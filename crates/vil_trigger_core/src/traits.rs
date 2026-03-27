// =============================================================================
// vil_trigger_core::traits — TriggerSource
// =============================================================================
//
// Async trait implemented by every VIL Phase 3 trigger crate.
//
// The on_event callback uses `Arc<dyn Fn(TriggerEvent) + Send + Sync>` so the
// trait remains dyn-compatible (no generic type parameter on start).
//
// Requires async-trait = "0.1" for stable async fn in traits.
// =============================================================================

use std::sync::Arc;

use async_trait::async_trait;

use crate::types::{TriggerEvent, TriggerFault};

/// Shared callback type for trigger event emission.
///
/// Callers wrap their handler in an Arc and pass it to `start`.
pub type EventCallback = Arc<dyn Fn(TriggerEvent) + Send + Sync>;

/// Core trait for all VIL trigger sources.
///
/// Implementations produce `TriggerEvent` values by calling `on_event`
/// whenever the external event source fires.
///
/// The trait is dyn-compatible — no generic parameters on any method.
#[async_trait]
pub trait TriggerSource: Send + Sync {
    /// Unique, stable identifier for this trigger kind (e.g. `"cdc"`, `"email"`).
    fn kind(&self) -> &'static str;

    /// Start watching for events.
    ///
    /// Loops until the source is stopped or a fatal fault occurs,
    /// calling `on_event` for each event.
    async fn start(&self, on_event: EventCallback) -> Result<(), TriggerFault>;

    /// Pause the trigger without destroying source state.
    async fn pause(&self) -> Result<(), TriggerFault>;

    /// Resume after a previous `pause`.
    async fn resume(&self) -> Result<(), TriggerFault>;

    /// Gracefully stop the trigger and release all resources.
    async fn stop(&self) -> Result<(), TriggerFault>;
}
