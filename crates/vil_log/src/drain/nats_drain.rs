// =============================================================================
// vil_log::drain::nats_drain — NATS Drain (Cross-Host Fan-Out)
// =============================================================================
//
// Publishes log events to NATS subjects for cross-host log aggregation.
//
// Subject mapping:
//   {prefix}.access    ← AccessLog events
//   {prefix}.app       ← AppLog events
//   {prefix}.system    ← SystemLog events
//   {prefix}.security  ← SecurityLog events
//   {prefix}.ai        ← AiLog events
//   {prefix}.db        ← DbLog events
//   {prefix}.mq        ← MqLog events
//
// Wire format: MsgPack (header + payload bytes)
//
// Requires feature: nats-drain
// =============================================================================

use async_trait::async_trait;

use crate::drain::traits::LogDrain;
use crate::types::{LogCategory, LogSlot};

/// NATS drain configuration.
#[derive(Debug, Clone)]
pub struct NatsConfig {
    /// NATS server URL (e.g., "nats://localhost:4222")
    pub url: String,
    /// Subject prefix (e.g., "vil.logs")
    /// Events published to: {prefix}.{category}
    pub subject_prefix: String,
}

impl Default for NatsConfig {
    fn default() -> Self {
        Self {
            url: "nats://localhost:4222".to_string(),
            subject_prefix: "vil.logs".to_string(),
        }
    }
}

/// Wire format for a single log event over NATS.
#[derive(serde::Serialize)]
struct NatsLogMessage {
    event_id_high: u64,
    event_id_low: u64,
    trace_id: u64,
    tenant_id: u64,
    process_id: u64,
    timestamp_ns: u64,
    level: u8,
    category: u8,
    subcategory: u8,
    version: u8,
    service_hash: u32,
    handler_hash: u32,
    node_hash: u32,
    payload: Vec<u8>,
}

impl NatsLogMessage {
    fn from_slot(slot: &LogSlot) -> Self {
        let h = &slot.header;
        Self {
            event_id_high: (h.event_id >> 64) as u64,
            event_id_low: h.event_id as u64,
            trace_id: h.trace_id,
            tenant_id: h.tenant_id,
            process_id: h.process_id,
            timestamp_ns: h.timestamp_ns,
            level: h.level,
            category: h.category,
            subcategory: h.subcategory,
            version: h.version,
            service_hash: h.service_hash,
            handler_hash: h.handler_hash,
            node_hash: h.node_hash,
            payload: slot.payload.to_vec(),
        }
    }
}

fn category_subject(prefix: &str, category: u8) -> String {
    let name = match LogCategory::from(category) {
        LogCategory::Access => "access",
        LogCategory::App => "app",
        LogCategory::System => "system",
        LogCategory::Security => "security",
        LogCategory::Ai => "ai",
        LogCategory::Db => "db",
        LogCategory::Mq => "mq",
    };
    format!("{}.{}", prefix, name)
}

/// NATS log drain for cross-host fan-out.
///
/// Publishes each log event to a category-specific NATS subject
/// using MsgPack wire format.
pub struct NatsDrain {
    config: NatsConfig,
    client: Option<async_nats::Client>,
}

impl NatsDrain {
    /// Create a new NATS drain. Connection is established lazily on first flush.
    pub fn new(config: NatsConfig) -> Self {
        Self {
            config,
            client: None,
        }
    }

    /// Ensure connected, lazily.
    async fn ensure_connected(
        &mut self,
    ) -> Result<&async_nats::Client, Box<dyn std::error::Error + Send + Sync>> {
        if self.client.is_none() {
            let client = async_nats::connect(&self.config.url).await?;
            self.client = Some(client);
        }
        Ok(self.client.as_ref().unwrap())
    }
}

#[async_trait]
impl LogDrain for NatsDrain {
    fn name(&self) -> &'static str {
        "nats"
    }

    async fn flush(
        &mut self,
        batch: &[LogSlot],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if batch.is_empty() {
            return Ok(());
        }

        let client = self.ensure_connected().await?.clone();

        for slot in batch {
            let subject = category_subject(&self.config.subject_prefix, slot.header.category);
            let msg = NatsLogMessage::from_slot(slot);
            let bytes = rmp_serde::to_vec_named(&msg)?;
            client.publish(subject, bytes.into()).await?;
        }

        client.flush().await?;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(client) = self.client.take() {
            client.flush().await?;
        }
        Ok(())
    }
}
