// =============================================================================
// vil_mq_rabbitmq::config — RabbitMQ connection configuration
// =============================================================================

use serde::{Deserialize, Serialize};

/// RabbitMQ connection and routing configuration.
///
/// Config types use External layout profile (setup-time only, not on hot path).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RabbitConfig {
    /// AMQP URI, e.g. "amqp://guest:guest@localhost:5672/%2F"
    pub uri: String,
    /// Default exchange name for publish operations.
    pub exchange: String,
    /// Default queue name for consume operations.
    pub queue: String,
    /// Consumer tag prefix.
    #[serde(default = "default_consumer_tag")]
    pub consumer_tag: String,
    /// Connection timeout in milliseconds.
    #[serde(default = "default_timeout_ms")]
    pub connection_timeout_ms: u64,
    /// Prefetch count for QoS.
    #[serde(default = "default_prefetch")]
    pub prefetch_count: u16,
}

fn default_consumer_tag() -> String { "vil-consumer".into() }
fn default_timeout_ms() -> u64 { 5_000 }
fn default_prefetch() -> u16 { 10 }

impl RabbitConfig {
    pub fn new(uri: &str, exchange: &str, queue: &str) -> Self {
        Self {
            uri: uri.into(),
            exchange: exchange.into(),
            queue: queue.into(),
            consumer_tag: default_consumer_tag(),
            connection_timeout_ms: default_timeout_ms(),
            prefetch_count: default_prefetch(),
        }
    }

    pub fn with_prefetch(mut self, count: u16) -> Self {
        self.prefetch_count = count;
        self
    }

    pub fn with_consumer_tag(mut self, tag: &str) -> Self {
        self.consumer_tag = tag.into();
        self
    }
}
