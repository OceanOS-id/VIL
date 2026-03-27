// =============================================================================
// vil_trigger_email::state — connector state for ServiceProcess health metrics
// =============================================================================

use vil_connector_macros::connector_state;

/// Live state metrics for the email trigger.
#[connector_state]
pub struct EmailTriggerState {
    /// Total emails received and emitted as events.
    pub emails_received: u64,
    /// Total IMAP IDLE reconnections performed.
    pub reconnections: u64,
    /// Total IMAP errors encountered.
    pub imap_errors: u64,
    /// Timestamp (ns) of the most recent email event (0 if none).
    pub last_event_ns: u64,
}
