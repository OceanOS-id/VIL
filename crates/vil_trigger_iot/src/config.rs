// =============================================================================
// vil_trigger_iot::config — IotConfig
// =============================================================================
//
// Configuration for the MQTT IoT trigger.
// =============================================================================

/// Configuration for the VIL IoT trigger (MQTT).
///
/// # Example YAML
/// ```yaml
/// iot:
///   mqtt_host: "broker.example.com"
///   port: 1883
///   topic: "sensors/+/temperature"
///   client_id: "vil-iot-trigger-01"
/// ```
#[derive(Debug, Clone)]
pub struct IotConfig {
    /// MQTT broker hostname or IP.
    pub mqtt_host: String,
    /// MQTT broker port (default: 1883).
    pub port: u16,
    /// MQTT topic filter to subscribe (supports wildcards).
    pub topic: String,
    /// MQTT client identifier (must be unique per broker).
    pub client_id: String,
}

impl IotConfig {
    /// Construct a new `IotConfig`.
    pub fn new(
        mqtt_host: impl Into<String>,
        port: u16,
        topic: impl Into<String>,
        client_id: impl Into<String>,
    ) -> Self {
        Self {
            mqtt_host: mqtt_host.into(),
            port,
            topic: topic.into(),
            client_id: client_id.into(),
        }
    }
}
