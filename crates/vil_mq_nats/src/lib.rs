// =============================================================================
// VIL NATS Adapter — Core, JetStream, KV Store
// =============================================================================

pub mod bridge;
pub mod client;
pub mod config;
pub mod health;
pub mod jetstream;
pub mod kv;
pub mod metrics;

pub use bridge::NatsBridge;
pub use client::NatsClient;
pub use config::NatsConfig;
pub use jetstream::JetStreamClient;
pub use kv::KvStore;
