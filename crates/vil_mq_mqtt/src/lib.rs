// =============================================================================
// VIL MQTT Adapter — Publish, Subscribe, Tri-Lane Bridge
// =============================================================================
//
// MQTT integration for IoT workloads.
// Note: Full rumqttc integration requires adding the rumqttc crate.
// This provides the API surface and bridge architecture.

pub mod config;
pub mod client;
pub mod bridge;

pub use config::{MqttConfig, QoS};
pub use client::MqttClient;
pub use bridge::MqttBridge;
