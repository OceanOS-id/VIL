// =============================================================================
// vil_trigger_evm::process — create_trigger helper
// =============================================================================

use std::sync::Arc;

use vil_trigger_core::traits::TriggerSource;

use crate::config::EvmConfig;
use crate::source::EvmTrigger;

/// Create an `EvmTrigger` wrapped in `Arc<dyn TriggerSource>`.
pub fn create_trigger(config: EvmConfig) -> Arc<dyn TriggerSource> {
    Arc::new(EvmTrigger::new(config))
}
