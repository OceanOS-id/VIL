// =============================================================================
// vil_trigger_core::process — create_trigger helper
// =============================================================================
//
// Convenience constructor for wrapping a TriggerSource in an Arc for shared
// ownership within a VIL ServiceProcess context.
// =============================================================================

use std::sync::Arc;

use crate::traits::TriggerSource;

/// Wrap any `TriggerSource` implementor in an `Arc<dyn TriggerSource>`.
///
/// # Usage
/// ```ignore
/// use vil_trigger_core::process::create_trigger;
/// let shared = create_trigger(MyConcreteTrigger::new(config));
/// ```
pub fn create_trigger<T: TriggerSource + 'static>(source: T) -> Arc<dyn TriggerSource> {
    Arc::new(source)
}
