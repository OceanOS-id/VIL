use std::sync::atomic::{AtomicU64, Ordering};

pub struct NatsMetrics {
    pub published: AtomicU64,
    pub received: AtomicU64,
    pub js_published: AtomicU64,
    pub js_consumed: AtomicU64,
    pub kv_puts: AtomicU64,
    pub kv_gets: AtomicU64,
    pub bridged: AtomicU64,
}

impl NatsMetrics {
    pub fn new() -> Self {
        Self {
            published: AtomicU64::new(0), received: AtomicU64::new(0),
            js_published: AtomicU64::new(0), js_consumed: AtomicU64::new(0),
            kv_puts: AtomicU64::new(0), kv_gets: AtomicU64::new(0),
            bridged: AtomicU64::new(0),
        }
    }

    pub fn to_prometheus(&self) -> String {
        format!(
            "vil_nats_published {}\nvil_nats_received {}\nvil_nats_js_published {}\nvil_nats_js_consumed {}\nvil_nats_kv_puts {}\nvil_nats_kv_gets {}\nvil_nats_bridged {}\n",
            self.published.load(Ordering::Relaxed), self.received.load(Ordering::Relaxed),
            self.js_published.load(Ordering::Relaxed), self.js_consumed.load(Ordering::Relaxed),
            self.kv_puts.load(Ordering::Relaxed), self.kv_gets.load(Ordering::Relaxed),
            self.bridged.load(Ordering::Relaxed),
        )
    }
}

impl Default for NatsMetrics { fn default() -> Self { Self::new() } }
