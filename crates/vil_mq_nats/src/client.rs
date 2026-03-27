// =============================================================================
// NATS Core Client — publish, subscribe, request/reply (real async-nats)
// =============================================================================

use crate::config::NatsConfig;
use bytes::Bytes;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use futures::StreamExt;

/// NATS message.
#[derive(Debug, Clone)]
pub struct NatsMessage {
    pub subject: String,
    pub payload: Bytes,
    pub reply_to: Option<String>,
}

/// NATS subscription handle (wraps real async-nats subscriber).
pub struct NatsSubscription {
    inner: async_nats::Subscriber,
    subject: String,
}

impl NatsSubscription {
    pub async fn next(&mut self) -> Option<NatsMessage> {
        let msg = self.inner.next().await?;
        Some(NatsMessage {
            subject: msg.subject.to_string(),
            payload: msg.payload.clone(),
            reply_to: msg.reply.as_ref().map(|s| s.to_string()),
        })
    }
    pub fn subject(&self) -> &str { &self.subject }
}

/// NATS core client backed by real async-nats connection.
pub struct NatsClient {
    client: async_nats::Client,
    config: NatsConfig,
    connected: AtomicBool,
    published: AtomicU64,
    received: AtomicU64,
}

impl NatsClient {
    /// Connect to a real NATS server.
    pub async fn connect(config: NatsConfig) -> Result<Self, String> {
        tracing::info!(url = %config.url, name = %config.client_name, "nats connecting (real async-nats)");

        let opts = async_nats::ConnectOptions::new()
            .name(&config.client_name)
            .max_reconnects(Some(config.max_reconnects as usize));

        // Apply credentials if configured
        let opts = if let Some(ref creds) = config.credentials {
            if let Some(ref token) = creds.token {
                opts.token(token.clone())
            } else if let (Some(ref user), Some(ref pass)) = (&creds.username, &creds.password) {
                opts.user_and_password(user.clone(), pass.clone())
            } else {
                opts
            }
        } else {
            opts
        };

        let client = opts.connect(&config.url).await
            .map_err(|e| format!("NATS connect failed: {}", e))?;

        tracing::info!(url = %config.url, "nats connected successfully");
        Ok(Self {
            client,
            config,
            connected: AtomicBool::new(true),
            published: AtomicU64::new(0),
            received: AtomicU64::new(0),
        })
    }

    /// Publish a message to a subject.
    pub async fn publish(&self, subject: &str, payload: &[u8]) -> Result<(), String> {
        if !self.connected.load(Ordering::Relaxed) {
            return Err("NATS not connected".into());
        }
        self.client.publish(subject.to_string(), Bytes::copy_from_slice(payload)).await
            .map_err(|e| format!("NATS publish failed: {}", e))?;
        self.published.fetch_add(1, Ordering::Relaxed);
        tracing::debug!(subject = %subject, size = payload.len(), "nats publish");
        Ok(())
    }

    /// Subscribe to a subject (supports wildcards: *, >).
    pub async fn subscribe(&self, subject: &str) -> Result<NatsSubscription, String> {
        let sub = self.client.subscribe(subject.to_string()).await
            .map_err(|e| format!("NATS subscribe failed: {}", e))?;
        tracing::info!(subject = %subject, "nats subscribe");
        Ok(NatsSubscription { inner: sub, subject: subject.to_string() })
    }

    /// Request/reply (sends and waits for one response).
    pub async fn request(&self, subject: &str, payload: &[u8]) -> Result<NatsMessage, String> {
        let resp = self.client.request(subject.to_string(), Bytes::copy_from_slice(payload)).await
            .map_err(|e| format!("NATS request failed: {}", e))?;
        self.published.fetch_add(1, Ordering::Relaxed);
        self.received.fetch_add(1, Ordering::Relaxed);
        Ok(NatsMessage {
            subject: resp.subject.to_string(),
            payload: resp.payload.clone(),
            reply_to: resp.reply.as_ref().map(|s| s.to_string()),
        })
    }

    /// Disconnect.
    pub async fn disconnect(&self) {
        self.connected.store(false, Ordering::Relaxed);
        // Flush pending messages before disconnect
        let _ = self.client.flush().await;
        tracing::info!("nats disconnected");
    }

    pub fn is_connected(&self) -> bool { self.connected.load(Ordering::Relaxed) }
    pub fn published_count(&self) -> u64 { self.published.load(Ordering::Relaxed) }
    pub fn received_count(&self) -> u64 { self.received.load(Ordering::Relaxed) }
    pub fn config(&self) -> &NatsConfig { &self.config }

    /// Access the underlying async-nats client for advanced use cases.
    pub fn inner(&self) -> &async_nats::Client { &self.client }
}
