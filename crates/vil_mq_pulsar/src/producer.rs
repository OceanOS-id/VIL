// =============================================================================
// vil_mq_pulsar::producer — PulsarProducer: send messages
// =============================================================================

use crate::client::PulsarClient;
use crate::error::PulsarFault;
use pulsar::{Producer, TokioExecutor};
use vil_log::dict::register_str;

/// Pulsar producer bound to a topic.
///
/// This crate spawns no internal threads for the producer.
pub struct PulsarProducer {
    inner: Producer<TokioExecutor>,
    topic_fqn: String,
    topic_hash: u32,
}

impl PulsarProducer {
    /// Create a new producer for the given topic (short name, FQN is built internally).
    pub async fn new(client: &PulsarClient, topic: &str) -> Result<Self, PulsarFault> {
        let topic_fqn = client.config().topic_fqn(topic);
        let topic_hash = register_str(&topic_fqn);

        let inner = client
            .inner
            .producer()
            .with_topic(&topic_fqn)
            .with_name("vil-producer")
            .build()
            .await
            .map_err(|_| PulsarFault::ProducerFailed { topic_hash })?;

        Ok(Self {
            inner,
            topic_fqn,
            topic_hash,
        })
    }

    /// Send a raw byte payload to the topic.
    pub async fn send(&mut self, payload: &[u8]) -> Result<(), PulsarFault> {
        let __start = std::time::Instant::now();
        let topic_hash = self.topic_hash;

        self.inner
            .send_non_blocking(payload.to_vec())
            .await
            .map_err(|_| PulsarFault::SendFailed {
                topic_hash,
                error_code: 0,
            })?
            .await
            .map_err(|_| PulsarFault::SendFailed {
                topic_hash,
                error_code: 1,
            })?;

        let __elapsed = __start.elapsed();
        {
            use vil_log::{mq_log, types::MqPayload};
            mq_log!(
                Info,
                MqPayload {
                    broker_hash: register_str("pulsar"),
                    topic_hash,
                    message_bytes: payload.len() as u32,
                    e2e_latency_ns: __elapsed.as_nanos() as u64,
                    op_type: 0, // publish
                    ..Default::default()
                }
            );
        }

        Ok(())
    }

    pub fn topic_fqn(&self) -> &str {
        &self.topic_fqn
    }
}
