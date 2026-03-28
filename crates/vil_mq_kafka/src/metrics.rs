use std::sync::atomic::{AtomicU64, Ordering};

pub struct KafkaMetrics {
    pub produced: AtomicU64,
    pub consumed: AtomicU64,
    pub bridged: AtomicU64,
    pub errors: AtomicU64,
}

impl KafkaMetrics {
    pub fn new() -> Self {
        Self {
            produced: AtomicU64::new(0),
            consumed: AtomicU64::new(0),
            bridged: AtomicU64::new(0),
            errors: AtomicU64::new(0),
        }
    }

    pub fn to_prometheus(&self) -> String {
        format!(
            "vil_kafka_produced_total {}\nvil_kafka_consumed_total {}\nvil_kafka_bridged_total {}\nvil_kafka_errors_total {}\n",
            self.produced.load(Ordering::Relaxed),
            self.consumed.load(Ordering::Relaxed),
            self.bridged.load(Ordering::Relaxed),
            self.errors.load(Ordering::Relaxed),
        )
    }
}

impl Default for KafkaMetrics {
    fn default() -> Self {
        Self::new()
    }
}
