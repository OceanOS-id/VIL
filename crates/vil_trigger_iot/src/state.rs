// =============================================================================
// vil_trigger_iot::state — connector state for ServiceProcess health metrics
// =============================================================================

use vil_connector_macros::connector_state;

/// Live state metrics for the IoT trigger.
#[connector_state]
pub struct IotTriggerState {
    /// Total MQTT device events received.
    pub device_events: u64,
    /// Total MQTT broker reconnections.
    pub reconnections: u64,
    /// Total MQTT event loop errors.
    pub mqtt_errors: u64,
    /// Timestamp (ns) of the most recent device event (0 if none).
    pub last_event_ns: u64,
}
