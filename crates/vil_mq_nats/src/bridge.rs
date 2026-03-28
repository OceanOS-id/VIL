use std::sync::atomic::{AtomicU64, Ordering};
use vil_log::app_log;

/// NATS ↔ Tri-Lane SHM Bridge.
pub struct NatsBridge {
    bridged: AtomicU64,
    target: String,
}

impl NatsBridge {
    pub fn new(target_service: &str) -> Self {
        Self {
            bridged: AtomicU64::new(0),
            target: target_service.into(),
        }
    }

    pub async fn bridge(&self, subject: &str, payload: &[u8]) {
        self.bridged.fetch_add(1, Ordering::Relaxed);
        app_log!(Debug, "nats.bridge", { subject: vil_log::dict::register_str(subject) as u64, size: payload.len() as u64 });
    }

    pub fn bridged_count(&self) -> u64 {
        self.bridged.load(Ordering::Relaxed)
    }
    pub fn target(&self) -> &str {
        &self.target
    }
}
