// =============================================================================
// VIL NATS Adapter — Core, JetStream, KV Store
// =============================================================================

pub mod config;
pub mod client;
pub mod jetstream;
pub mod kv;
pub mod bridge;
pub mod metrics;
pub mod health;

pub use config::NatsConfig;
pub use client::NatsClient;
pub use jetstream::JetStreamClient;
pub use kv::KvStore;
pub use bridge::NatsBridge;
