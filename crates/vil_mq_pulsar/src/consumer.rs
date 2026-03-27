// =============================================================================
// vil_mq_pulsar::consumer — PulsarConsumer: receive messages
// =============================================================================

use crate::client::PulsarClient;
use crate::error::PulsarFault;
use bytes::Bytes;
use futures::TryStreamExt;
use pulsar::{message::proto::MessageIdData, Consumer, SubType, TokioExecutor};
use vil_log::dict::register_str;

/// A received Pulsar message.
#[derive(Debug)]
pub struct PulsarMessage {
    /// Raw payload bytes.
    pub payload: Bytes,
    /// Hash of the topic FQN.
    pub topic_hash: u32,
    /// Message ID for ack.
    pub message_id: MessageIdData,
}

/// Pulsar consumer bound to a topic and subscription.
///
/// This crate spawns 1 internal tokio task when `start()` is called.
/// Add 1 per active consumer to `LogConfig.threads`.
pub struct PulsarConsumer {
    inner: Consumer<Vec<u8>, TokioExecutor>,
    topic_hash: u32,
    topic_fqn: String,
}

impl PulsarConsumer {
    /// Create a new consumer for the given topic and subscription name.
    pub async fn new(
        client: &PulsarClient,
        topic: &str,
        subscription: &str,
    ) -> Result<Self, PulsarFault> {
        let topic_fqn = client.config().topic_fqn(topic);
        let topic_hash = register_str(&topic_fqn);
        let subscription_hash = register_str(subscription);

        let inner: Consumer<Vec<u8>, TokioExecutor> = client
            .inner
            .consumer()
            .with_topic(&topic_fqn)
            .with_subscription(subscription)
            .with_subscription_type(SubType::Shared)
            .build()
            .await
            .map_err(|_| PulsarFault::ConsumerFailed {
                topic_hash,
                subscription_hash,
            })?;

        let _ = subscription_hash;

        Ok(Self {
            inner,
            topic_hash,
            topic_fqn,
        })
    }

    /// Receive the next message (blocks until one is available).
    pub async fn receive(&mut self) -> Result<PulsarMessage, PulsarFault> {
        let __start = std::time::Instant::now();
        let topic_hash = self.topic_hash;

        let msg = self
            .inner
            .try_next()
            .await
            .map_err(|_| PulsarFault::ReceiveFailed { topic_hash })?
            .ok_or(PulsarFault::ReceiveFailed { topic_hash })?;

        let payload_len = msg.payload.data.len() as u32;
        let message_id = msg.message_id().clone();

        let __elapsed = __start.elapsed();
        {
            use vil_log::{mq_log, types::MqPayload};
            mq_log!(Info, MqPayload {
                broker_hash:    register_str("pulsar"),
                topic_hash,
                message_bytes:  payload_len,
                e2e_latency_us: __elapsed.as_micros() as u32,
                op_type:        1, // consume
                ..Default::default()
            });
        }

        Ok(PulsarMessage {
            payload: Bytes::copy_from_slice(&msg.payload.data),
            topic_hash,
            message_id,
        })
    }

    /// Acknowledge a previously received message.
    pub async fn ack(&mut self, message_id: &MessageIdData) -> Result<(), PulsarFault> {
        let __start = std::time::Instant::now();
        let topic_hash = self.topic_hash;

        self.inner
            .ack_with_id(self.topic_fqn.as_str(), message_id.clone())
            .await
            .map_err(|_| PulsarFault::AckFailed { topic_hash })?;

        let __elapsed = __start.elapsed();
        {
            use vil_log::{mq_log, types::MqPayload};
            mq_log!(Info, MqPayload {
                broker_hash:    register_str("pulsar"),
                topic_hash,
                message_bytes:  0,
                e2e_latency_us: __elapsed.as_micros() as u32,
                op_type:        2, // ack
                ..Default::default()
            });
        }

        Ok(())
    }

    pub fn topic_fqn(&self) -> &str {
        &self.topic_fqn
    }
}
