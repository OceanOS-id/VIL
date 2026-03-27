// =============================================================================
// vil_trigger_iot::process — create_trigger helper
// =============================================================================

use std::sync::Arc;

use vil_trigger_core::traits::TriggerSource;

use crate::config::IotConfig;
use crate::source::IotTrigger;

/// Create an `IotTrigger` wrapped in `Arc<dyn TriggerSource>`.
pub fn create_trigger(config: IotConfig) -> Arc<dyn TriggerSource> {
    Arc::new(IotTrigger::new(config))
}
