// =============================================================================
// vil_trigger_webhook::state — connector state for ServiceProcess health metrics
// =============================================================================

use vil_connector_macros::connector_state;

/// Live state metrics for the webhook trigger.
#[connector_state]
pub struct WebhookTriggerState {
    /// Total webhook requests received successfully.
    pub webhooks_received: u64,
    /// Total requests rejected due to invalid HMAC signature.
    pub signature_rejections: u64,
    /// Total body read failures.
    pub body_errors: u64,
    /// Timestamp (ns) of the most recent webhook (0 if none).
    pub last_event_ns: u64,
}
