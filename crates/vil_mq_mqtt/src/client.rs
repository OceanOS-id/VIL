// =============================================================================
// MQTT Client — real rumqttc AsyncClient
// =============================================================================

use crate::config::{MqttConfig, QoS};
use rumqttc::{AsyncClient, MqttOptions, QoS as MqttQoS};
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::time::Duration;

/// Map our QoS enum to rumqttc QoS.
fn to_mqtt_qos(qos: QoS) -> MqttQoS {
    match qos {
        QoS::AtMostOnce => MqttQoS::AtMostOnce,
        QoS::AtLeastOnce => MqttQoS::AtLeastOnce,
        QoS::ExactlyOnce => MqttQoS::ExactlyOnce,
    }
}

/// MQTT client backed by real rumqttc AsyncClient.
pub struct MqttClient {
    client: AsyncClient,
    config: MqttConfig,
    connected: AtomicBool,
    published: AtomicU64,
    received: AtomicU64,
}

impl MqttClient {
    pub async fn new(config: MqttConfig) -> Result<Self, String> {
        let client_id = config.client_id.clone().unwrap_or_else(|| "vil-mqtt-client".into());

        // Parse host from broker_url (strip mqtt:// prefix if present)
        let host = config.broker_url
            .trim_start_matches("mqtt://")
            .trim_start_matches("mqtts://")
            .trim_start_matches("tcp://")
            .to_string();

        let mut opts = MqttOptions::new(&client_id, &host, config.port);
        opts.set_keep_alive(Duration::from_secs(config.keepalive_secs));

        // Apply credentials if configured
        if let (Some(ref user), Some(ref pass)) = (&config.username, &config.password) {
            opts.set_credentials(user, pass);
        }

        let (client, mut eventloop) = AsyncClient::new(opts, 100);

        // Spawn the event loop in the background
        tokio::spawn(async move {
            loop {
                match eventloop.poll().await {
                    Ok(_event) => { /* event processed */ }
                    Err(e) => {
                        tracing::warn!(error = %e, "mqtt eventloop error, retrying...");
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        });

        tracing::info!(broker = %config.broker_url, port = config.port, client_id = %client_id, "mqtt client created (real rumqttc)");

        Ok(Self {
            client,
            config,
            connected: AtomicBool::new(true),
            published: AtomicU64::new(0),
            received: AtomicU64::new(0),
        })
    }

    /// Publish a message to a topic.
    pub async fn publish(&self, topic: &str, payload: &[u8], qos: QoS) -> Result<(), String> {
        if !self.connected.load(Ordering::Relaxed) {
            return Err("MQTT not connected".into());
        }
        let __mq_start = std::time::Instant::now();
        let result = self.client.publish(topic, to_mqtt_qos(qos), false, payload).await
            .map_err(|e| format!("MQTT publish failed: {}", e));
        if result.is_ok() {
            self.published.fetch_add(1, Ordering::Relaxed);
        }
        let __elapsed = __mq_start.elapsed();
        {
            use vil_log::{mq_log, types::MqPayload};
            mq_log!(Info, MqPayload {
                broker_hash: register_str("mqtt"),
                topic_hash: register_str(topic),
                message_bytes: payload.len() as u32,
                e2e_latency_us: __elapsed.as_micros() as u32,
                op_type: 0,
                ..Default::default()
            });
        }
        tracing::debug!(topic = %topic, qos = ?qos, size = payload.len(), "mqtt publish");
        result
    }

    /// Subscribe to a topic pattern.
    pub async fn subscribe(&self, topic_filter: &str) -> Result<(), String> {
        if !self.connected.load(Ordering::Relaxed) {
            return Err("MQTT not connected".into());
        }
        self.client.subscribe(topic_filter, to_mqtt_qos(self.config.qos)).await
            .map_err(|e| format!("MQTT subscribe failed: {}", e))?;
        tracing::info!(topic = %topic_filter, "mqtt subscribe");
        Ok(())
    }

    /// Disconnect.
    pub async fn disconnect(&self) {
        self.connected.store(false, Ordering::Relaxed);
        let _ = self.client.disconnect().await;
        tracing::info!("mqtt disconnected");
    }

    pub fn is_connected(&self) -> bool { self.connected.load(Ordering::Relaxed) }
    pub fn published_count(&self) -> u64 { self.published.load(Ordering::Relaxed) }
    pub fn received_count(&self) -> u64 { self.received.load(Ordering::Relaxed) }
    pub fn config(&self) -> &MqttConfig { &self.config }
}
