// =============================================================================
// vil_trigger_iot::events — connector events emitted on Data Lane
// =============================================================================

use vil_connector_macros::connector_event;

/// Emitted when an IoT device event (MQTT message) is received.
#[connector_event]
pub struct DeviceEvent {
    /// FxHash of the MQTT topic string.
    pub topic_hash: u32,
    /// FxHash of the client ID string.
    pub client_hash: u32,
    /// Message payload size in bytes.
    pub payload_bytes: u32,
    /// MQTT QoS level: 0, 1, or 2.
    pub qos: u8,
    /// Wall-clock timestamp in nanoseconds (UNIX_EPOCH).
    pub timestamp_ns: u64,
}
