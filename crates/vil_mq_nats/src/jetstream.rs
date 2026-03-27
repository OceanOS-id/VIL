// =============================================================================
// NATS JetStream — Persistent streaming, durable consumers (real async-nats)
// =============================================================================

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use futures::StreamExt;

/// JetStream stream configuration (user-facing config, mapped to async-nats types).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    pub name: String,
    pub subjects: Vec<String>,
    #[serde(default = "default_retention")]
    pub retention: String,
    #[serde(default = "default_max_msgs")]
    pub max_msgs: i64,
    #[serde(default = "default_max_bytes")]
    pub max_bytes: i64,
}

fn default_retention() -> String { "limits".into() }
fn default_max_msgs() -> i64 { -1 }
fn default_max_bytes() -> i64 { -1 }

/// JetStream consumer configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerConfig {
    pub durable_name: Option<String>,
    pub filter_subject: Option<String>,
    #[serde(default = "default_ack")]
    pub ack_policy: String,
    #[serde(default = "default_deliver")]
    pub deliver_policy: String,
}

fn default_ack() -> String { "explicit".into() }
fn default_deliver() -> String { "all".into() }

/// JetStream message (with ack support).
pub struct JsMessage {
    pub subject: String,
    pub payload: Bytes,
    pub stream: String,
    pub sequence: u64,
    inner: Option<async_nats::jetstream::Message>,
    acked: AtomicBool,
}

impl JsMessage {
    /// Acknowledge the message.
    pub async fn ack(&self) -> Result<(), String> {
        if let Some(ref inner) = self.inner {
            inner.ack().await.map_err(|e| format!("jetstream ack failed: {}", e))?;
        }
        self.acked.store(true, Ordering::Relaxed);
        tracing::debug!(stream = %self.stream, seq = self.sequence, "jetstream ack");
        Ok(())
    }

    /// Negative-acknowledge (request redelivery).
    pub async fn nack(&self) -> Result<(), String> {
        if let Some(ref inner) = self.inner {
            inner.ack_with(async_nats::jetstream::AckKind::Nak(None)).await
                .map_err(|e| format!("jetstream nack failed: {}", e))?;
        }
        tracing::debug!(stream = %self.stream, seq = self.sequence, "jetstream nack");
        Ok(())
    }

    pub fn is_acked(&self) -> bool { self.acked.load(Ordering::Relaxed) }
}

/// JetStream consumer handle (wraps real async-nats pull consumer stream).
pub struct JsConsumer {
    inner: async_nats::jetstream::consumer::pull::Stream,
    config: ConsumerConfig,
    stream_name: String,
    messages_received: AtomicU64,
}

impl JsConsumer {
    pub async fn next(&mut self) -> Option<JsMessage> {
        let msg = self.inner.next().await?;
        match msg {
            Ok(msg) => {
                self.messages_received.fetch_add(1, Ordering::Relaxed);
                let sequence = msg.info().ok().map(|i| i.stream_sequence).unwrap_or(0);
                Some(JsMessage {
                    subject: msg.subject.to_string(),
                    payload: msg.payload.clone(),
                    stream: self.stream_name.clone(),
                    sequence,
                    inner: Some(msg),
                    acked: AtomicBool::new(false),
                })
            }
            Err(e) => {
                tracing::warn!(error = %e, "jetstream consumer error");
                None
            }
        }
    }

    pub fn messages_received(&self) -> u64 { self.messages_received.load(Ordering::Relaxed) }
    pub fn config(&self) -> &ConsumerConfig { &self.config }
}

/// JetStream client backed by real async-nats JetStream context.
pub struct JetStreamClient {
    js: async_nats::jetstream::Context,
    stream_names: dashmap::DashMap<String, StreamConfig>,
}

impl JetStreamClient {
    /// Create a JetStream client from an async-nats client.
    pub fn new(client: &async_nats::Client) -> Self {
        Self {
            js: async_nats::jetstream::new(client.clone()),
            stream_names: dashmap::DashMap::new(),
        }
    }

    /// Create a persistent stream.
    pub async fn create_stream(&self, config: StreamConfig) -> Result<(), String> {
        let retention = match config.retention.as_str() {
            "workqueue" => async_nats::jetstream::stream::RetentionPolicy::WorkQueue,
            "interest" => async_nats::jetstream::stream::RetentionPolicy::Interest,
            _ => async_nats::jetstream::stream::RetentionPolicy::Limits,
        };

        self.js.get_or_create_stream(async_nats::jetstream::stream::Config {
            name: config.name.clone(),
            subjects: config.subjects.clone(),
            retention,
            max_messages: config.max_msgs,
            max_bytes: config.max_bytes,
            ..Default::default()
        }).await.map_err(|e| format!("jetstream create_stream failed: {}", e))?;

        tracing::info!(stream = %config.name, subjects = ?config.subjects, "jetstream stream created");
        self.stream_names.insert(config.name.clone(), config);
        Ok(())
    }

    /// Create a durable consumer on a stream.
    pub async fn create_consumer(&self, stream: &str, config: ConsumerConfig) -> Result<JsConsumer, String> {
        let stream_handle = self.js.get_stream(stream).await
            .map_err(|e| format!("jetstream get_stream failed: {}", e))?;

        let deliver_policy = match config.deliver_policy.as_str() {
            "last" => async_nats::jetstream::consumer::DeliverPolicy::Last,
            "new" => async_nats::jetstream::consumer::DeliverPolicy::New,
            _ => async_nats::jetstream::consumer::DeliverPolicy::All,
        };

        let ack_policy = match config.ack_policy.as_str() {
            "none" => async_nats::jetstream::consumer::AckPolicy::None,
            "all" => async_nats::jetstream::consumer::AckPolicy::All,
            _ => async_nats::jetstream::consumer::AckPolicy::Explicit,
        };

        let consumer_cfg = async_nats::jetstream::consumer::pull::Config {
            durable_name: config.durable_name.clone(),
            filter_subject: config.filter_subject.clone().unwrap_or_default(),
            deliver_policy,
            ack_policy,
            ..Default::default()
        };

        let consumer = stream_handle.create_consumer(consumer_cfg).await
            .map_err(|e| format!("jetstream create_consumer failed: {}", e))?;

        let messages = consumer.messages().await
            .map_err(|e| format!("jetstream consumer messages failed: {}", e))?;

        tracing::info!(stream = %stream, consumer = ?config.durable_name, "jetstream consumer created");
        Ok(JsConsumer {
            inner: messages,
            config,
            stream_name: stream.to_string(),
            messages_received: AtomicU64::new(0),
        })
    }

    /// Publish to a JetStream subject.
    pub async fn publish(&self, subject: &str, payload: &[u8]) -> Result<u64, String> {
        let ack = self.js.publish(subject.to_string(), Bytes::copy_from_slice(payload)).await
            .map_err(|e| format!("jetstream publish failed: {}", e))?;
        let ack = ack.await
            .map_err(|e| format!("jetstream publish ack failed: {}", e))?;
        let seq = ack.sequence;
        tracing::debug!(subject = %subject, seq = seq, "jetstream publish");
        Ok(seq)
    }

    /// Get stream names.
    pub fn streams(&self) -> Vec<String> {
        self.stream_names.iter().map(|e| e.key().clone()).collect()
    }

    pub fn stream_count(&self) -> usize { self.stream_names.len() }

    /// Access the underlying JetStream context for advanced use cases.
    pub fn inner(&self) -> &async_nats::jetstream::Context { &self.js }
}
