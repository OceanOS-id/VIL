// =============================================================================
// vil_mq_pubsub::client — PubSubClient: publish and subscribe
// =============================================================================

use crate::config::PubSubConfig;
use crate::error::PubSubFault;
use bytes::Bytes;
use google_cloud_googleapis::pubsub::v1::PubsubMessage as GcpMessage;
use google_cloud_pubsub::client::{Client, ClientConfig};
use google_cloud_pubsub::publisher::Publisher;
use google_cloud_pubsub::subscription::Subscription;
use google_cloud_pubsub::topic::Topic;
use vil_log::dict::register_str;

/// A received Pub/Sub message.
///
/// Call `ack()` on this struct to acknowledge the message directly.
#[derive(Debug)]
pub struct PubSubMessage {
    /// Raw payload bytes.
    pub payload: Bytes,
    /// Ack ID (for tracking/logging).
    pub ack_id: String,
    /// Hash of the topic path.
    pub topic_hash: u32,
    /// Hash of the subscription path.
    pub subscription_hash: u32,
    /// Inner received message (holds ack handle).
    inner: google_cloud_pubsub::subscriber::ReceivedMessage,
}

impl PubSubMessage {
    /// Acknowledge this message to remove it from the subscription.
    pub async fn ack(self) -> Result<(), PubSubFault> {
        let subscription_hash = self.subscription_hash;
        let __start = std::time::Instant::now();

        self.inner
            .ack()
            .await
            .map_err(|_| PubSubFault::AckFailed { subscription_hash })?;

        let __elapsed = __start.elapsed();
        {
            use vil_log::{mq_log, types::MqPayload};
            mq_log!(
                Info,
                MqPayload {
                    broker_hash: register_str("pubsub"),
                    topic_hash: self.topic_hash,
                    message_bytes: 0,
                    e2e_latency_us: __elapsed.as_micros() as u32,
                    op_type: 2, // ack
                    ..Default::default()
                }
            );
        }

        Ok(())
    }
}

/// Google Cloud Pub/Sub client.
///
/// This crate spawns no internal threads for publish or pull-based subscribe.
/// When using streaming `receive()`, add 1 per active stream to `LogConfig.threads`.
pub struct PubSubClient {
    inner: Client,
    config: PubSubConfig,
}

impl PubSubClient {
    /// Create a new PubSubClient.
    ///
    /// Credentials are loaded from ADC (Application Default Credentials) or
    /// GOOGLE_APPLICATION_CREDENTIALS env var. For emulator testing, set
    /// `PUBSUB_EMULATOR_HOST` or use `config.with_emulator()`.
    pub async fn new(config: PubSubConfig) -> Result<Self, PubSubFault> {
        let project_hash = register_str(&config.project_id);

        if let Some(ref host) = config.emulator_host {
            // The SDK picks up PUBSUB_EMULATOR_HOST automatically.
            std::env::set_var("PUBSUB_EMULATOR_HOST", host);
        }

        let client_config = ClientConfig::default()
            .with_auth()
            .await
            .map_err(|_| PubSubFault::ClientInitFailed { project_hash })?;

        let inner = Client::new(client_config)
            .await
            .map_err(|_| PubSubFault::ClientInitFailed { project_hash })?;

        let _ = project_hash;
        Ok(Self { inner, config })
    }

    /// Publish a raw byte payload to the configured topic.
    pub async fn publish(&self, data: &[u8]) -> Result<(), PubSubFault> {
        let __start = std::time::Instant::now();
        let topic_path = self.config.topic_path();
        let topic_hash = register_str(&topic_path);

        let topic: Topic = self.inner.topic(&self.config.topic);
        let publisher: Publisher = topic.new_publisher(None);

        let msg = GcpMessage {
            data: data.to_vec(),
            ..Default::default()
        };

        let awaiter = publisher.publish(msg).await;

        awaiter
            .get()
            .await
            .map_err(|_| PubSubFault::PublishFailed {
                topic_hash,
                status_code: 0,
            })?;

        let __elapsed = __start.elapsed();
        {
            use vil_log::{mq_log, types::MqPayload};
            mq_log!(
                Info,
                MqPayload {
                    broker_hash: register_str("pubsub"),
                    topic_hash,
                    message_bytes: data.len() as u32,
                    e2e_latency_us: __elapsed.as_micros() as u32,
                    op_type: 0, // publish
                    ..Default::default()
                }
            );
        }

        Ok(())
    }

    /// Pull messages from the configured subscription.
    ///
    /// Returns a list of `PubSubMessage`. Each message carries its own `ack()` handle.
    pub async fn subscribe(&self) -> Result<Vec<PubSubMessage>, PubSubFault> {
        let __start = std::time::Instant::now();
        let sub_path = self.config.subscription_path();
        let subscription_hash = register_str(&sub_path);
        let topic_hash = register_str(&self.config.topic_path());

        let subscription: Subscription = self.inner.subscription(&self.config.subscription);

        let raw_msgs = subscription
            .pull(self.config.max_messages, None)
            .await
            .map_err(|_| PubSubFault::ReceiveFailed { subscription_hash })?;

        let count = raw_msgs.len() as u32;
        let __elapsed = __start.elapsed();
        {
            use vil_log::{mq_log, types::MqPayload};
            mq_log!(
                Info,
                MqPayload {
                    broker_hash: register_str("pubsub"),
                    topic_hash,
                    message_bytes: count,
                    e2e_latency_us: __elapsed.as_micros() as u32,
                    op_type: 1, // consume
                    ..Default::default()
                }
            );
        }

        let msgs = raw_msgs
            .into_iter()
            .map(|m| {
                let payload = Bytes::copy_from_slice(&m.message.data);
                let ack_id = m.ack_id().to_string();
                PubSubMessage {
                    payload,
                    ack_id,
                    topic_hash,
                    subscription_hash,
                    inner: m,
                }
            })
            .collect();

        Ok(msgs)
    }

    pub fn config(&self) -> &PubSubConfig {
        &self.config
    }
}
