// =============================================================================
// VIL MQTT Adapter — Publish, Subscribe, Tri-Lane Bridge
// =============================================================================
//
// MQTT integration for IoT workloads.
// Note: Full rumqttc integration requires adding the rumqttc crate.
// This provides the API surface and bridge architecture.

pub mod bridge;
pub mod client;
pub mod config;

pub use bridge::MqttBridge;
pub use client::MqttClient;
pub use config::{MqttConfig, QoS};
