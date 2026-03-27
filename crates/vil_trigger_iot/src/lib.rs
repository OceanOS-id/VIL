// =============================================================================
// vil_trigger_iot — VIL Phase 3 IoT MQTT Trigger
// =============================================================================
//
// MQTT subscription-based IoT device event trigger using rumqttc.
//
// Modules:
//   config  — IotConfig (mqtt_host, port, topic, client_id)
//   source  — IotTrigger implements TriggerSource
//   error   — IotFault plain enum
//   process — create_trigger convenience constructor
//
// No println!, tracing, or log crate usage — COMPLIANCE.md §8.
// =============================================================================

pub mod config;
pub mod error;
pub mod process;
pub mod source;

pub use config::IotConfig;
pub use error::IotFault;
pub use source::IotTrigger;
