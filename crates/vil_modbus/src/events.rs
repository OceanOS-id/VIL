// =============================================================================
// vil_modbus::events — connector events emitted on Data Lane
// =============================================================================

use vil_connector_macros::connector_event;

/// Emitted when registers are successfully read from a Modbus device.
#[connector_event]
pub struct RegisterRead {
    /// Starting register address.
    pub address: u16,
    /// Number of registers read.
    pub count: u16,
    /// Round-trip latency in microseconds.
    pub latency_us: u32,
    /// Wall-clock timestamp in nanoseconds (UNIX_EPOCH).
    pub timestamp_ns: u64,
}

/// Emitted when a register is successfully written to a Modbus device.
#[connector_event]
pub struct RegisterWritten {
    /// Register address that was written.
    pub address: u16,
    /// Number of registers written.
    pub count: u16,
    /// Round-trip latency in microseconds.
    pub latency_us: u32,
    /// Wall-clock timestamp in nanoseconds (UNIX_EPOCH).
    pub timestamp_ns: u64,
}
