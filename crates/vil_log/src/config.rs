// =============================================================================
// vil_log::config — LogConfig
// =============================================================================
//
// Plain struct configuration. No YAML parsing — callers build it directly
// or via builder methods.
// =============================================================================

use crate::types::LogLevel;

/// Configuration for the VIL log system.
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// Total ring capacity across all stripes (must be > 0).
    /// Divided evenly across stripes. Each stripe rounded to power-of-2.
    pub ring_slots: usize,

    /// Minimum level to emit. Events below this level are dropped at the macro.
    pub level: LogLevel,

    /// Maximum slots to drain per batch flush.
    pub batch_size: usize,

    /// Maximum milliseconds to wait before flushing a partial batch.
    pub flush_interval_ms: u64,

    /// Expected application thread count.
    /// Determines stripe count: 1 SPSC ring per thread (no contention).
    ///
    /// - `Some(n)` → exactly `n` stripes (rounded to power-of-2, max 32)
    /// - `None` → auto-detect from `available_parallelism()`
    ///
    /// Guidelines:
    ///   Web server:      threads = tokio worker threads (typically num_cpus)
    ///   Data pipeline:   threads = pipeline parallelism (4-16)
    ///   CLI tool:        threads = 1 or 2
    ///   Microservice:    threads = None (auto-detect)
    pub threads: Option<usize>,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            ring_slots:        8192,
            level:             LogLevel::Info,
            batch_size:        256,
            flush_interval_ms: 10,
            threads:           None, // auto-detect
        }
    }
}

impl LogConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ring_slots(mut self, n: usize) -> Self {
        self.ring_slots = n;
        self
    }

    pub fn level(mut self, l: LogLevel) -> Self {
        self.level = l;
        self
    }

    pub fn batch_size(mut self, n: usize) -> Self {
        self.batch_size = n;
        self
    }

    pub fn flush_interval_ms(mut self, ms: u64) -> Self {
        self.flush_interval_ms = ms;
        self
    }

    /// Set expected thread count. Determines stripe count for optimal performance.
    pub fn threads(mut self, n: usize) -> Self {
        self.threads = Some(n);
        self
    }
}
