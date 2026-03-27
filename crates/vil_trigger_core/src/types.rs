// =============================================================================
// vil_trigger_core::types — TriggerEvent + TriggerFault
// =============================================================================
//
// Plain struct/enum types used across all trigger crates.
// No thiserror, no String fields — COMPLIANCE.md §4.
// All string context stored as u32 FxHash via register_str().
// =============================================================================

/// Lightweight descriptor of a fired trigger event.
///
/// Emitted on the Trigger Lane when a trigger fires.
/// All fields are fixed-size (no heap) — fits in a cache line.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TriggerEvent {
    /// FxHash of the trigger kind string (e.g. "cdc", "email", "iot").
    pub kind_hash: u32,
    /// FxHash of the trigger source identifier (e.g. slot name, topic).
    pub source_hash: u32,
    /// Monotonic sequence number — increments per trigger instance.
    pub sequence: u64,
    /// Wall-clock timestamp in nanoseconds (UNIX_EPOCH).
    pub timestamp_ns: u64,
    /// Size of the associated event payload in bytes (0 if none).
    pub payload_bytes: u32,
    /// Operation code: 0=fire, 1=pause_ack, 2=resume_ack, 3=stop_ack.
    pub op: u8,
    /// Reserved for alignment.
    pub _pad: [u8; 3],
}

impl Default for TriggerEvent {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

/// Fault type for trigger infrastructure operations.
///
/// All variant fields are primitive — no heap allocation.
/// Use `register_str()` to convert string context to u32 hashes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerFault {
    /// The trigger source is unavailable (connection lost, etc.).
    SourceUnavailable {
        /// FxHash of the trigger kind string.
        kind_hash: u32,
        /// Numeric reason code (OS error or protocol-specific).
        reason_code: u32,
    },
    /// The trigger configuration is invalid.
    ConfigInvalid {
        /// FxHash of the invalid field name.
        field_hash: u32,
    },
    /// The trigger was rate-limited.
    RateLimited {
        /// FxHash of the trigger kind string.
        kind_hash: u32,
        /// Current events per second at time of limiting.
        events_per_sec: u32,
    },
    /// An I/O error occurred during event consumption.
    IoError {
        /// FxHash of the trigger kind string.
        kind_hash: u32,
        /// OS error code.
        os_code: u32,
    },
    /// Authentication or authorization failed connecting to the source.
    AuthFailed {
        /// FxHash of the trigger kind string.
        kind_hash: u32,
    },
}

impl TriggerFault {
    /// Return a stable numeric code for log fields.
    pub fn as_error_code(&self) -> u32 {
        match self {
            TriggerFault::SourceUnavailable { .. } => 1,
            TriggerFault::ConfigInvalid { .. }     => 2,
            TriggerFault::RateLimited { .. }       => 3,
            TriggerFault::IoError { .. }           => 4,
            TriggerFault::AuthFailed { .. }        => 5,
        }
    }
}
