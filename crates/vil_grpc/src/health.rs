// =============================================================================
// VIL gRPC — Health Check Reporter
// =============================================================================

use std::sync::atomic::{AtomicBool, Ordering};

/// gRPC health status reporter.
pub struct HealthReporter {
    serving: AtomicBool,
}

impl HealthReporter {
    pub fn new() -> Self {
        Self { serving: AtomicBool::new(true) }
    }

    pub fn set_serving(&self, serving: bool) {
        self.serving.store(serving, Ordering::Relaxed);
    }

    pub fn is_serving(&self) -> bool {
        self.serving.load(Ordering::Relaxed)
    }
}

impl Default for HealthReporter {
    fn default() -> Self { Self::new() }
}
