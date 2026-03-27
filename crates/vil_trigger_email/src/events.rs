// =============================================================================
// vil_trigger_email::events — connector events emitted on Data Lane
// =============================================================================

use vil_connector_macros::connector_event;

/// Emitted when a new email message is received via IMAP IDLE.
#[connector_event]
pub struct EmailReceived {
    /// FxHash of the mailbox folder name.
    pub folder_hash: u32,
    /// IMAP sequence number of the received message.
    pub seq: u32,
    /// Approximate message size in bytes (from RFC822.SIZE, 0 if unavailable).
    pub message_bytes: u32,
    /// Wall-clock timestamp in nanoseconds (UNIX_EPOCH).
    pub timestamp_ns: u64,
}
