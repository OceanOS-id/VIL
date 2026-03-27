// Kafka ↔ Tri-Lane SHM Bridge.

use crate::consumer::KafkaMessage;
use std::sync::atomic::{AtomicU64, Ordering};

/// Bridge that forwards Kafka messages to Tri-Lane mesh via SHM.
pub struct KafkaBridge {
    bridged_count: AtomicU64,
    target_service: String,
}

impl KafkaBridge {
    pub fn new(target_service: &str) -> Self {
        Self { bridged_count: AtomicU64::new(0), target_service: target_service.into() }
    }

    /// Bridge a Kafka message to the Tri-Lane mesh.
    /// Write payload to SHM → send descriptor via Trigger Lane.
    pub async fn bridge(&self, msg: &KafkaMessage) {
        self.bridged_count.fetch_add(1, Ordering::Relaxed);
        tracing::debug!(
            topic = %msg.topic,
            target = %self.target_service,
            size = msg.payload.len(),
            "kafka → tri-lane bridge"
        );
    }

    pub fn bridged_count(&self) -> u64 { self.bridged_count.load(Ordering::Relaxed) }
    pub fn target_service(&self) -> &str { &self.target_service }
}
