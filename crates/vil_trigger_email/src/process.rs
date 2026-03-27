// =============================================================================
// vil_trigger_email::process — create_trigger helper
// =============================================================================

use std::sync::Arc;

use vil_trigger_core::traits::TriggerSource;

use crate::config::EmailConfig;
use crate::source::EmailTrigger;

/// Create an `EmailTrigger` wrapped in `Arc<dyn TriggerSource>`.
pub fn create_trigger(config: EmailConfig) -> Arc<dyn TriggerSource> {
    Arc::new(EmailTrigger::new(config))
}
