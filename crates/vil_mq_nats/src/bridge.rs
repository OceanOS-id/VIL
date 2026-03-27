use std::sync::atomic::{AtomicU64, Ordering};

/// NATS ↔ Tri-Lane SHM Bridge.
pub struct NatsBridge {
    bridged: AtomicU64,
    target: String,
}

impl NatsBridge {
    pub fn new(target_service: &str) -> Self {
        Self { bridged: AtomicU64::new(0), target: target_service.into() }
    }

    pub async fn bridge(&self, subject: &str, payload: &[u8]) {
        self.bridged.fetch_add(1, Ordering::Relaxed);
        tracing::debug!(subject = %subject, target = %self.target, size = payload.len(), "nats → tri-lane");
    }

    pub fn bridged_count(&self) -> u64 { self.bridged.load(Ordering::Relaxed) }
    pub fn target(&self) -> &str { &self.target }
}
