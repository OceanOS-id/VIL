// MQTT ↔ Tri-Lane bridge.

use std::sync::atomic::{AtomicU64, Ordering};

pub struct MqttBridge {
    bridged_count: AtomicU64,
    target_service: String,
}

impl MqttBridge {
    pub fn new(target_service: &str) -> Self {
        Self {
            bridged_count: AtomicU64::new(0),
            target_service: target_service.into(),
        }
    }

    pub async fn bridge(&self, topic: &str, payload: &[u8]) {
        self.bridged_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn bridged_count(&self) -> u64 {
        self.bridged_count.load(Ordering::Relaxed)
    }
}
