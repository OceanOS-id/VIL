// =============================================================================
// VIL Server Profiler — Runtime performance profiling
// =============================================================================
//
// Provides runtime performance metrics beyond per-handler observability:
//   - Memory usage tracking (heap + SHM)
//   - Event loop latency monitoring
//   - SHM allocation efficiency
//   - Connection pool utilization
//
// Exposed via GET /admin/profile endpoint.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Server-wide performance profile.
pub struct ServerProfiler {
    start_time: Instant,
    /// Total bytes allocated in SHM
    shm_allocated_bytes: AtomicU64,
    /// Total SHM regions created
    shm_regions_created: AtomicU64,
    /// Peak concurrent connections
    peak_connections: AtomicU64,
    /// Current concurrent connections
    current_connections: AtomicU64,
    /// Total requests served since startup
    total_requests: AtomicU64,
    /// Total bytes received
    bytes_received: AtomicU64,
    /// Total bytes sent
    bytes_sent: AtomicU64,
}

impl ServerProfiler {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            shm_allocated_bytes: AtomicU64::new(0),
            shm_regions_created: AtomicU64::new(0),
            peak_connections: AtomicU64::new(0),
            current_connections: AtomicU64::new(0),
            total_requests: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
        }
    }

    pub fn record_shm_alloc(&self, bytes: u64) {
        self.shm_allocated_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn record_shm_region(&self) {
        self.shm_regions_created.fetch_add(1, Ordering::Relaxed);
    }

    pub fn connection_opened(&self) {
        let current = self.current_connections.fetch_add(1, Ordering::Relaxed) + 1;
        let peak = self.peak_connections.load(Ordering::Relaxed);
        if current > peak {
            self.peak_connections.store(current, Ordering::Relaxed);
        }
    }

    pub fn connection_closed(&self) {
        self.current_connections.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn record_request(&self, bytes_in: u64, bytes_out: u64) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.bytes_received.fetch_add(bytes_in, Ordering::Relaxed);
        self.bytes_sent.fetch_add(bytes_out, Ordering::Relaxed);
    }

    /// Generate a performance profile snapshot.
    pub fn snapshot(&self) -> ProfileSnapshot {
        let uptime = self.start_time.elapsed();
        let total_reqs = self.total_requests.load(Ordering::Relaxed);
        let rps = if uptime.as_secs() > 0 {
            total_reqs as f64 / uptime.as_secs_f64()
        } else {
            0.0
        };

        ProfileSnapshot {
            uptime_secs: uptime.as_secs(),
            total_requests: total_reqs,
            requests_per_sec: rps,
            current_connections: self.current_connections.load(Ordering::Relaxed),
            peak_connections: self.peak_connections.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            shm_allocated_bytes: self.shm_allocated_bytes.load(Ordering::Relaxed),
            shm_regions_created: self.shm_regions_created.load(Ordering::Relaxed),
            memory_rss_bytes: get_rss_bytes(),
        }
    }
}

impl Default for ServerProfiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance profile snapshot (serializable).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProfileSnapshot {
    pub uptime_secs: u64,
    pub total_requests: u64,
    pub requests_per_sec: f64,
    pub current_connections: u64,
    pub peak_connections: u64,
    pub bytes_received: u64,
    pub bytes_sent: u64,
    pub shm_allocated_bytes: u64,
    pub shm_regions_created: u64,
    pub memory_rss_bytes: u64,
}

/// Get current RSS (Resident Set Size) memory usage.
/// Reads /proc/self/statm on Linux.
fn get_rss_bytes() -> u64 {
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/self/statm") {
            if let Some(rss_pages) = content.split_whitespace().nth(1) {
                if let Ok(pages) = rss_pages.parse::<u64>() {
                    return pages * 4096; // page size = 4KB
                }
            }
        }
    }
    0
}
