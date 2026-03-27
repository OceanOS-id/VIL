// =============================================================================
// vil_modbus::state — connector state for ServiceProcess health metrics
// =============================================================================

use vil_connector_macros::connector_state;

/// Live state metrics for the Modbus client.
#[connector_state]
pub struct ModbusClientState {
    /// Total register read operations completed.
    pub registers_read: u64,
    /// Total register write operations completed.
    pub registers_written: u64,
    /// Total Modbus errors (timeouts, exceptions, mismatches).
    pub modbus_errors: u64,
    /// Average round-trip latency in microseconds.
    pub avg_latency_us: u32,
}
