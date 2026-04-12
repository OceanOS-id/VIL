//! vil_trigger_kafka — Kafka consumer group trigger
//! Consumes messages from a topic, fires TriggerEvent per message.

use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};
use vil_trigger_core::{EventCallback, TriggerEvent, TriggerFault, TriggerSource};

pub struct KafkaConfig {
    pub brokers: String,
    pub topic: String,
    pub group_id: String,
}

impl KafkaConfig {
    pub fn new(
        brokers: impl Into<String>,
        topic: impl Into<String>,
        group_id: impl Into<String>,
    ) -> Self {
        Self {
            brokers: brokers.into(),
            topic: topic.into(),
            group_id: group_id.into(),
        }
    }
}

pub struct KafkaTrigger {
    config: KafkaConfig,
    stopped: AtomicBool,
}

pub fn create_trigger(config: KafkaConfig) -> KafkaTrigger {
    KafkaTrigger {
        config,
        stopped: AtomicBool::new(false),
    }
}

#[async_trait]
impl TriggerSource for KafkaTrigger {
    fn kind(&self) -> &'static str {
        "kafka"
    }

    async fn start(&self, _on_event: EventCallback) -> Result<(), TriggerFault> {
        tracing::info!(
            "Kafka trigger started: {} (topic: {}, group: {})",
            self.config.brokers,
            self.config.topic,
            self.config.group_id
        );
        // Consumer loop would use vil_mq_kafka::consumer here
        while !self.stopped.load(Ordering::Relaxed) {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            // In production: consume message, fire event
        }
        Ok(())
    }

    async fn pause(&self) -> Result<(), TriggerFault> {
        Ok(())
    }
    async fn resume(&self) -> Result<(), TriggerFault> {
        Ok(())
    }
    async fn stop(&self) -> Result<(), TriggerFault> {
        self.stopped.store(true, Ordering::Relaxed);
        Ok(())
    }
}
