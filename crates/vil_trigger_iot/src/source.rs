// =============================================================================
// vil_trigger_iot::source — IotTrigger
// =============================================================================
//
// MQTT subscription-based IoT trigger using rumqttc.
//
// On every MQTT Publish packet from the subscribed topic:
//   1. Times the event loop poll.
//   2. Emits mq_log! with timing, message size, and topic hash.
//   3. Calls on_event callback with a TriggerEvent.
//
// No println!, tracing, or log crate — COMPLIANCE.md §8.
// =============================================================================

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};

use vil_log::dict::register_str;
use vil_log::{mq_log, types::MqPayload};

use vil_trigger_core::traits::{EventCallback, TriggerSource};
use vil_trigger_core::types::{TriggerEvent, TriggerFault};

use crate::config::IotConfig;
use crate::error::IotFault;

/// MQTT subscription IoT trigger.
///
/// Subscribes to a topic on an MQTT broker using `rumqttc` and fires a
/// `TriggerEvent` on every matching `PUBLISH` packet.
pub struct IotTrigger {
    config: IotConfig,
    paused: Arc<AtomicBool>,
    sequence: Arc<AtomicU64>,
    /// Cached hashes for hot-path log emission.
    host_hash: u32,
    topic_hash: u32,
    kind_hash: u32,
}

impl IotTrigger {
    /// Create a new `IotTrigger` from config.
    pub fn new(config: IotConfig) -> Self {
        let host_addr = format!("{}:{}", config.mqtt_host, config.port);
        let host_hash = register_str(&host_addr);
        let topic_hash = register_str(&config.topic);
        let kind_hash = register_str("iot");
        Self {
            config,
            paused: Arc::new(AtomicBool::new(false)),
            sequence: Arc::new(AtomicU64::new(0)),
            host_hash,
            topic_hash,
            kind_hash,
        }
    }

    fn map_fault(f: IotFault, kind_hash: u32) -> TriggerFault {
        TriggerFault::SourceUnavailable {
            kind_hash,
            reason_code: f.as_error_code(),
        }
    }

    async fn run_loop(&self, on_event: &EventCallback) -> Result<(), IotFault> {
        let host_hash = self.host_hash;
        let topic_hash = self.topic_hash;
        let kind_hash = self.kind_hash;

        let mut opts = MqttOptions::new(
            &self.config.client_id,
            &self.config.mqtt_host,
            self.config.port,
        );
        opts.set_keep_alive(std::time::Duration::from_secs(30));

        let (client, mut event_loop) = AsyncClient::new(opts, 64);

        // Subscribe to the topic.
        client
            .subscribe(&self.config.topic, QoS::AtLeastOnce)
            .await
            .map_err(|_| IotFault::SubscribeFailed {
                topic_hash,
                return_code: 0,
            })?;

        loop {
            if self.paused.load(Ordering::Relaxed) {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                continue;
            }

            let start = std::time::Instant::now();
            let notification = event_loop
                .poll()
                .await
                .map_err(|_| IotFault::EventLoopError {
                    host_hash,
                    kind_code: 1,
                })?;

            if let Event::Incoming(Packet::Publish(publish)) = notification {
                let elapsed = start.elapsed();
                let seq = self.sequence.fetch_add(1, Ordering::Relaxed);
                let msg_len = publish.payload.len() as u32;

                // Emit mq_log! on every MQTT message arrival.
                mq_log!(
                    Info,
                    MqPayload {
                        broker_hash: host_hash,
                        topic_hash: register_str(&publish.topic),
                        group_hash: kind_hash,
                        offset: seq,
                        message_bytes: msg_len,
                        e2e_latency_ns: elapsed.as_nanos() as u64,
                        op_type: 1, // consume
                        partition: publish.qos as u8,
                        retries: 0,
                        compression: 0,
                        ..MqPayload::default()
                    }
                );

                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64;

                on_event(TriggerEvent {
                    kind_hash,
                    source_hash: topic_hash,
                    sequence: seq,
                    timestamp_ns: ts,
                    payload_bytes: msg_len,
                    op: 0,
                    _pad: [0; 3],
                });
            }
        }
    }
}

#[async_trait]
impl TriggerSource for IotTrigger {
    fn kind(&self) -> &'static str {
        "iot"
    }

    async fn start(&self, on_event: EventCallback) -> Result<(), TriggerFault> {
        self.run_loop(&on_event)
            .await
            .map_err(|e| Self::map_fault(e, self.kind_hash))
    }

    async fn pause(&self) -> Result<(), TriggerFault> {
        self.paused.store(true, Ordering::Relaxed);
        Ok(())
    }

    async fn resume(&self) -> Result<(), TriggerFault> {
        self.paused.store(false, Ordering::Relaxed);
        Ok(())
    }

    async fn stop(&self) -> Result<(), TriggerFault> {
        self.paused.store(true, Ordering::Relaxed);
        Ok(())
    }
}
