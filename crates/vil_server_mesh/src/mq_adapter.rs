// =============================================================================
// VIL Server Mesh — Message Queue Adapter
// =============================================================================
//
// Provides a trait-based message queue abstraction for vil-server.
// Supports NATS, Kafka, and other MQ systems via pluggable adapters.
//
// The adapter bridges external message queues into the Tri-Lane mesh:
//   MQ message arrives → write to SHM → dispatch via Trigger/Data Lane
//   Tri-Lane message → serialize → publish to MQ
//
// This enables vil-server to participate in event-driven architectures
// while maintaining zero-copy benefits for local processing.

use async_trait::async_trait;
use bytes::Bytes;
use serde::Deserialize;

/// Message queue message.
#[derive(Debug, Clone)]
pub struct MqMessage {
    /// Topic/subject the message was published to
    pub subject: String,
    /// Message payload
    pub payload: Bytes,
    /// Optional reply-to subject (for request/reply)
    pub reply_to: Option<String>,
    /// Message headers
    pub headers: Vec<(String, String)>,
}

impl MqMessage {
    pub fn new(subject: impl Into<String>, payload: impl Into<Bytes>) -> Self {
        Self {
            subject: subject.into(),
            payload: payload.into(),
            reply_to: None,
            headers: Vec::new(),
        }
    }

    pub fn with_reply(mut self, reply_to: impl Into<String>) -> Self {
        self.reply_to = Some(reply_to.into());
        self
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((key.into(), value.into()));
        self
    }
}

/// Message queue adapter trait.
///
/// Implement this trait to integrate NATS, Kafka, RabbitMQ, etc.
/// with the vil-server mesh.
#[async_trait]
pub trait MqAdapter: Send + Sync + 'static {
    type Error: std::error::Error + Send + Sync;

    /// Connect to the message queue.
    async fn connect(&mut self) -> Result<(), Self::Error>;

    /// Publish a message to a subject/topic.
    async fn publish(&self, msg: MqMessage) -> Result<(), Self::Error>;

    /// Subscribe to a subject/topic.
    /// Returns a receiver that yields messages.
    async fn subscribe(&self, subject: &str) -> Result<MqSubscription, Self::Error>;

    /// Request/reply pattern.
    async fn request(&self, msg: MqMessage, timeout_ms: u64) -> Result<MqMessage, Self::Error>;

    /// Disconnect from the message queue.
    async fn disconnect(&mut self) -> Result<(), Self::Error>;

    /// Check if connected.
    fn is_connected(&self) -> bool;
}

/// Subscription handle — yields messages from a subscribed topic.
pub struct MqSubscription {
    rx: tokio::sync::mpsc::Receiver<MqMessage>,
}

impl MqSubscription {
    pub fn new(rx: tokio::sync::mpsc::Receiver<MqMessage>) -> Self {
        Self { rx }
    }

    /// Receive the next message.
    pub async fn recv(&mut self) -> Option<MqMessage> {
        self.rx.recv().await
    }
}

/// NATS adapter configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct NatsConfig {
    /// NATS server URL(s)
    pub urls: Vec<String>,
    /// Optional credentials (user:pass or token)
    pub credentials: Option<String>,
    /// Connection name
    pub name: Option<String>,
    /// Max reconnect attempts
    pub max_reconnects: Option<usize>,
}

impl NatsConfig {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            urls: vec![url.into()],
            credentials: None,
            name: Some("vil-server".to_string()),
            max_reconnects: Some(60),
        }
    }
}

/// NATS adapter (stub — requires `async-nats` crate).
///
/// To use with a real NATS server, add `async-nats` to your dependencies
/// and implement the MqAdapter trait using the NATS client.
pub struct NatsAdapter {
    config: NatsConfig,
    connected: bool,
}

impl NatsAdapter {
    pub fn new(config: NatsConfig) -> Self {
        Self {
            config,
            connected: false,
        }
    }

    pub fn config(&self) -> &NatsConfig {
        &self.config
    }
}

#[async_trait]
impl MqAdapter for NatsAdapter {
    type Error = MqError;

    async fn connect(&mut self) -> Result<(), Self::Error> {
        {
            use vil_log::app_log;
            app_log!(Info, "mesh.nats.connecting", {});
        }
        // Stub: real implementation would use async-nats client
        self.connected = true;
        Ok(())
    }

    async fn publish(&self, _msg: MqMessage) -> Result<(), Self::Error> {
        if !self.connected {
            return Err(MqError::NotConnected);
        }
        // debug-level: skip vil_log
        Ok(())
    }

    async fn subscribe(&self, subject: &str) -> Result<MqSubscription, Self::Error> {
        if !self.connected {
            return Err(MqError::NotConnected);
        }
        let (_tx, rx) = tokio::sync::mpsc::channel(256);
        {
            use vil_log::app_log;
            app_log!(Info, "mesh.nats.subscribe", { subject: subject });
        }
        Ok(MqSubscription::new(rx))
    }

    async fn request(&self, msg: MqMessage, _timeout_ms: u64) -> Result<MqMessage, Self::Error> {
        if !self.connected {
            return Err(MqError::NotConnected);
        }
        // Stub: return empty reply
        Ok(MqMessage::new(msg.subject, Bytes::new()))
    }

    async fn disconnect(&mut self) -> Result<(), Self::Error> {
        self.connected = false;
        {
            use vil_log::app_log;
            app_log!(Info, "mesh.nats.disconnected", {});
        }
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}

/// Message queue errors.
#[derive(Debug)]
pub enum MqError {
    NotConnected,
    ConnectionFailed(String),
    PublishFailed(String),
    SubscribeFailed(String),
    Timeout,
}

impl std::fmt::Display for MqError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MqError::NotConnected => write!(f, "Not connected to message queue"),
            MqError::ConnectionFailed(e) => write!(f, "Connection failed: {}", e),
            MqError::PublishFailed(e) => write!(f, "Publish failed: {}", e),
            MqError::SubscribeFailed(e) => write!(f, "Subscribe failed: {}", e),
            MqError::Timeout => write!(f, "Request timed out"),
        }
    }
}

impl std::error::Error for MqError {}
