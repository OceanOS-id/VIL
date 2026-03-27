// =============================================================================
// VIL Kafka Adapter — Producer, Consumer, Tri-Lane Bridge
// =============================================================================
//
// Real Kafka integration for vil-server.
// Note: Full rdkafka integration requires librdkafka C library.
// This implementation provides the API surface and Tri-Lane bridge.
// Production: add `rdkafka = { version = "0.36", features = ["cmake-build"] }`

pub mod config;
pub mod producer;
pub mod consumer;
pub mod bridge;
pub mod metrics;

pub use config::KafkaConfig;
pub use producer::KafkaProducer;
pub use consumer::KafkaConsumer;
pub use bridge::KafkaBridge;
