// =============================================================================
// vil_trigger_webhook::process — create_trigger helper
// =============================================================================

use std::sync::Arc;

use vil_trigger_core::traits::TriggerSource;

use crate::config::WebhookConfig;
use crate::source::WebhookTrigger;

/// Create a `WebhookTrigger` wrapped in `Arc<dyn TriggerSource>`.
pub fn create_trigger(config: WebhookConfig) -> Arc<dyn TriggerSource> {
    Arc::new(WebhookTrigger::new(config))
}
