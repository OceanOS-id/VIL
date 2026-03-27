// =============================================================================
// vil_trigger_cdc::process — create_trigger helper
// =============================================================================
//
// Convenience constructor for wrapping a CdcTrigger ready for use as a VIL
// ServiceProcess component.
// =============================================================================

use std::sync::Arc;

use vil_trigger_core::traits::TriggerSource;

use crate::config::CdcConfig;
use crate::source::CdcTrigger;

/// Create a `CdcTrigger` wrapped in an `Arc<dyn TriggerSource>`.
///
/// # Usage
/// ```ignore
/// use vil_trigger_cdc::process::create_trigger;
/// let trigger = create_trigger(CdcConfig::new("host=...", "slot", "pub"));
/// ```
pub fn create_trigger(config: CdcConfig) -> Arc<dyn TriggerSource> {
    Arc::new(CdcTrigger::new(config))
}
