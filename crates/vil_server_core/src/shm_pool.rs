// =============================================================================
// VIL Server — SHM Region Pool (Tuned for P99 Tail Latency)
// =============================================================================
//
// Pre-allocated shared memory pool for HTTP request/response bodies.
// Eliminates per-request region creation overhead (~0.5µs → 0µs).
//
// Architecture:
//   Startup: create one large region (configurable, default 64MB)
//   Request: bump-allocate within the region (O(1))
//   Reset:   amortized — check every N allocs, not every alloc
//
// P99 Tuning:
//   - Reset check amortized (every 256 allocs, not every alloc)
//   - CAS lock prevents thundering herd on reset
//   - Failed alloc → immediate reset + retry (no request drop)
//   - All thresholds configurable via ShmPoolConfig + env vars

use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::sync::Arc;

use vil_shm::ExchangeHeap;
use vil_types::RegionId;

/// Configuration for the SHM pool. All values configurable.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ShmPoolConfig {
    /// Pool capacity in bytes (default: 64MB)
    pub capacity: usize,
    /// Reset when utilization exceeds this % (default: 85)
    pub reset_threshold_pct: usize,
    /// Check utilization every N allocations (default: 256)
    pub check_interval: u64,
}

impl Default for ShmPoolConfig {
    fn default() -> Self {
        Self {
            capacity: 64 * 1024 * 1024,
            reset_threshold_pct: 85,
            check_interval: 256,
        }
    }
}

impl ShmPoolConfig {
    /// Small pool for development/testing
    pub fn dev() -> Self {
        Self { capacity: 8 * 1024 * 1024, reset_threshold_pct: 70, check_interval: 64 }
    }

    /// Large pool for high-throughput production
    pub fn production() -> Self {
        Self { capacity: 256 * 1024 * 1024, reset_threshold_pct: 90, check_interval: 1024 }
    }

    /// From environment variables:
    ///   VIL_SHM_CAPACITY_MB (default: 64)
    ///   VIL_SHM_RESET_PCT (default: 85)
    ///   VIL_SHM_CHECK_INTERVAL (default: 256)
    pub fn from_env() -> Self {
        let mut config = Self::default();
        if let Ok(mb) = std::env::var("VIL_SHM_CAPACITY_MB") {
            if let Ok(v) = mb.parse::<usize>() { config.capacity = v * 1024 * 1024; }
        }
        if let Ok(pct) = std::env::var("VIL_SHM_RESET_PCT") {
            if let Ok(v) = pct.parse::<usize>() { config.reset_threshold_pct = v.min(99); }
        }
        if let Ok(interval) = std::env::var("VIL_SHM_CHECK_INTERVAL") {
            if let Ok(v) = interval.parse::<u64>() { config.check_interval = v.max(1); }
        }
        config
    }
}

/// Pre-allocated SHM region pool for zero-copy HTTP I/O.
pub struct ShmPool {
    heap: Arc<ExchangeHeap>,
    region_id: RegionId,
    config: ShmPoolConfig,
    alloc_count: AtomicU64,
    reset_count: AtomicU64,
    retry_count: AtomicU64,
    resetting: AtomicBool,
}

impl ShmPool {
    /// Create a new SHM pool with configuration.
    pub fn new(heap: Arc<ExchangeHeap>, capacity: usize, reset_threshold: usize) -> Self {
        let config = ShmPoolConfig { capacity, reset_threshold_pct: reset_threshold, ..Default::default() };
        let region_id = heap.create_region("vil_http_pool", config.capacity);
        {
            use vil_log::system_log;
            use vil_log::types::SystemPayload;
            system_log!(Info, SystemPayload {
                event_type: 4, // startup
                mem_kb: (config.capacity / 1024) as u32,
                ..Default::default()
            });
        }
        Self {
            heap, region_id, config,
            alloc_count: AtomicU64::new(0),
            reset_count: AtomicU64::new(0),
            retry_count: AtomicU64::new(0),
            resetting: AtomicBool::new(false),
        }
    }

    /// Create from ShmPoolConfig.
    pub fn with_config(heap: Arc<ExchangeHeap>, config: ShmPoolConfig) -> Self {
        Self::new(heap, config.capacity, config.reset_threshold_pct)
    }

    /// Default pool — reads env vars, falls back to sensible defaults.
    pub fn default_pool(heap: Arc<ExchangeHeap>) -> Self {
        let config = ShmPoolConfig::from_env();
        Self::new(heap, config.capacity, config.reset_threshold_pct)
    }

    /// Allocate space and write data.
    ///
    /// P99 optimization: utilization check is amortized (every N allocs).
    pub fn alloc_and_write(&self, data: &[u8]) -> Option<(RegionId, vil_shm::Offset)> {
        let count = self.alloc_count.fetch_add(1, Ordering::Relaxed);

        // Amortized reset check — not every alloc
        if count % self.config.check_interval == 0 {
            self.maybe_reset();
        }

        // Try bump-allocate
        match self.heap.alloc_bytes(self.region_id, data.len(), 8) {
            Some(offset) => {
                self.heap.write_bytes(self.region_id, offset, data);
                Some((self.region_id, offset))
            }
            None => self.force_reset_and_retry(data),
        }
    }

    fn maybe_reset(&self) {
        if let Some(stats) = self.heap.region_stats(self.region_id) {
            let utilization = if stats.capacity > 0 { stats.used * 100 / stats.capacity } else { 0 };
            if utilization >= self.config.reset_threshold_pct {
                self.do_reset(utilization);
            }
        }
    }

    fn force_reset_and_retry(&self, data: &[u8]) -> Option<(RegionId, vil_shm::Offset)> {
        self.retry_count.fetch_add(1, Ordering::Relaxed);
        self.do_reset(100);
        let offset = self.heap.alloc_bytes(self.region_id, data.len(), 8)?;
        self.heap.write_bytes(self.region_id, offset, data);
        Some((self.region_id, offset))
    }

    fn do_reset(&self, utilization: usize) {
        if self.resetting.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_ok() {
            self.heap.reset_region(self.region_id);
            self.reset_count.fetch_add(1, Ordering::Relaxed);
            self.resetting.store(false, Ordering::Release);
            // debug-level: skip vil_log (below Info threshold)
        }
    }

    pub fn reset(&self) {
        self.heap.reset_region(self.region_id);
        self.reset_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn stats(&self) -> PoolStats {
        let (used, remaining) = self.heap.region_stats(self.region_id)
            .map(|s| (s.used, s.remaining)).unwrap_or((0, 0));
        PoolStats {
            capacity: self.config.capacity, used, remaining,
            utilization_pct: if self.config.capacity > 0 { used * 100 / self.config.capacity } else { 0 },
            total_allocs: self.alloc_count.load(Ordering::Relaxed),
            total_resets: self.reset_count.load(Ordering::Relaxed),
            total_retries: self.retry_count.load(Ordering::Relaxed),
            reset_threshold: self.config.reset_threshold_pct,
            check_interval: self.config.check_interval,
        }
    }

    pub fn config(&self) -> &ShmPoolConfig { &self.config }
    pub fn region_id(&self) -> RegionId { self.region_id }
    pub fn heap(&self) -> &Arc<ExchangeHeap> { &self.heap }
}

/// Pool statistics.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PoolStats {
    pub capacity: usize,
    pub used: usize,
    pub remaining: usize,
    pub utilization_pct: usize,
    pub total_allocs: u64,
    pub total_resets: u64,
    pub total_retries: u64,
    pub reset_threshold: usize,
    pub check_interval: u64,
}
