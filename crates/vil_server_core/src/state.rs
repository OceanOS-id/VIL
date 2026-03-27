// =============================================================================
// VIL Server State — Shared application state with Runtime integration
// =============================================================================

use std::sync::Arc;
use std::time::Instant;

use vil_obs::VilMetrics;
use vil_rt::VastarRuntimeWorld;
use vil_shm::ExchangeHeap;

use crate::capsule_handler::CapsuleRegistry;
use crate::shm_pool::ShmPool;
use crate::custom_metrics::CustomMetrics;
use crate::error_tracker::ErrorTracker;
use crate::hot_reload::ConfigReloader;
use crate::obs_middleware::HandlerMetricsRegistry;
use crate::otel::SpanCollector;
use crate::plugin_manager::PluginManager;
use crate::process::ProcessRegistry;
use crate::secrets::SecretResolver;
use crate::profiler::ServerProfiler;

/// Shared application state accessible from all handlers via Axum State extractor.
///
/// Holds:
/// - VastarRuntimeWorld for process registration and IPC
/// - ExchangeHeap for zero-copy SHM allocation
/// - VilMetrics for Prometheus metrics
/// - ProcessRegistry for handler process isolation
/// - HandlerMetricsRegistry for per-route auto-observability
#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    /// Server start time for uptime calculation
    start_time: Instant,

    /// VIL Prometheus metrics
    metrics: VilMetrics,

    /// VIL runtime (process-oriented IPC)
    runtime: Arc<VastarRuntimeWorld>,

    /// SHM exchange heap for zero-copy data passing
    shm: Arc<ExchangeHeap>,

    /// Pre-allocated SHM pool for HTTP I/O (performance-critical)
    shm_pool: Arc<ShmPool>,

    /// Process registry — each handler route = VIL process
    process_registry: Arc<ProcessRegistry>,

    /// Per-handler metrics (zero-instrumentation observability)
    handler_metrics: Arc<HandlerMetricsRegistry>,

    /// WASM capsule handler registry (hot-reload)
    capsule_registry: Arc<CapsuleRegistry>,

    /// OpenTelemetry span collector
    span_collector: Arc<SpanCollector>,

    /// Custom user-defined metrics
    custom_metrics: Arc<CustomMetrics>,

    /// Error tracker & aggregator
    error_tracker: Arc<ErrorTracker>,

    /// Server performance profiler
    profiler: Arc<ServerProfiler>,

    /// Hot config reloader
    config_reloader: Arc<ConfigReloader>,

    /// V6: Plugin manager
    plugin_manager: Arc<PluginManager>,

    /// Server name/identifier
    name: String,

    /// Server version
    version: String,
}

impl AppState {
    pub fn new(name: impl Into<String>) -> Self {
        let runtime = Arc::new(VastarRuntimeWorld::new());
        let shm = Arc::new(ExchangeHeap::new());
        let shm_pool = Arc::new(ShmPool::default_pool(shm.clone()));
        let process_registry = Arc::new(ProcessRegistry::new(runtime.clone()));
        let handler_metrics = Arc::new(HandlerMetricsRegistry::new());
        let capsule_registry = Arc::new(CapsuleRegistry::new());
        let span_collector = Arc::new(SpanCollector::default());
        let custom_metrics = Arc::new(CustomMetrics::new());
        let error_tracker = Arc::new(ErrorTracker::default());
        let profiler = Arc::new(ServerProfiler::new());
        let config_reloader = Arc::new(ConfigReloader::new());

        // V6: Plugin manager with secret resolver
        let plugins_dir = dirs_plugin_path();
        let secrets = Arc::new(SecretResolver::new(
            Some(&plugins_dir.join("..").join("secrets").join("encryption.key")),
        ));
        let plugin_manager = Arc::new(PluginManager::new(&plugins_dir, secrets));

        Self {
            inner: Arc::new(AppStateInner {
                start_time: Instant::now(),
                metrics: VilMetrics::new(),
                runtime,
                shm,
                shm_pool,
                process_registry,
                handler_metrics,
                capsule_registry,
                span_collector,
                custom_metrics,
                error_tracker,
                profiler,
                config_reloader,
                plugin_manager,
                name: name.into(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            }),
        }
    }

    /// Create AppState with a shared (SHM-backed) runtime.
    /// This enables cross-process zero-copy communication.
    pub fn new_shared(name: impl Into<String>) -> Result<Self, String> {
        let runtime = Arc::new(
            VastarRuntimeWorld::new_shared()
                .map_err(|e| format!("Failed to initialize SHM runtime: {:?}", e))?,
        );
        let shm = Arc::new(ExchangeHeap::new());
        let shm_pool = Arc::new(ShmPool::default_pool(shm.clone()));
        let process_registry = Arc::new(ProcessRegistry::new(runtime.clone()));
        let handler_metrics = Arc::new(HandlerMetricsRegistry::new());
        let capsule_registry = Arc::new(CapsuleRegistry::new());
        let span_collector = Arc::new(SpanCollector::default());
        let custom_metrics = Arc::new(CustomMetrics::new());
        let error_tracker = Arc::new(ErrorTracker::default());
        let profiler = Arc::new(ServerProfiler::new());
        let config_reloader = Arc::new(ConfigReloader::new());

        let plugins_dir = dirs_plugin_path();
        let secrets = Arc::new(SecretResolver::new(
            Some(&plugins_dir.join("..").join("secrets").join("encryption.key")),
        ));
        let plugin_manager = Arc::new(PluginManager::new(&plugins_dir, secrets));

        Ok(Self {
            inner: Arc::new(AppStateInner {
                start_time: Instant::now(),
                metrics: VilMetrics::new(),
                runtime,
                shm,
                shm_pool,
                process_registry,
                handler_metrics,
                capsule_registry,
                span_collector,
                custom_metrics,
                error_tracker,
                profiler,
                config_reloader,
                plugin_manager,
                name: name.into(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            }),
        })
    }

    /// Get server uptime in seconds
    pub fn uptime_secs(&self) -> u64 {
        self.inner.start_time.elapsed().as_secs()
    }

    /// Get VIL metrics collector
    pub fn metrics(&self) -> &VilMetrics {
        &self.inner.metrics
    }

    /// Get the VIL runtime
    pub fn runtime(&self) -> &Arc<VastarRuntimeWorld> {
        &self.inner.runtime
    }

    /// Get the SHM exchange heap
    pub fn shm(&self) -> &Arc<ExchangeHeap> {
        &self.inner.shm
    }

    /// Get the pre-allocated SHM pool (for ShmSlice, high-performance path)
    pub fn shm_pool(&self) -> &Arc<ShmPool> {
        &self.inner.shm_pool
    }

    /// Get the process registry (handler → VIL process mapping)
    pub fn process_registry(&self) -> &Arc<ProcessRegistry> {
        &self.inner.process_registry
    }

    /// Get per-handler metrics registry (zero-instrumentation observability)
    pub fn handler_metrics(&self) -> &Arc<HandlerMetricsRegistry> {
        &self.inner.handler_metrics
    }

    /// Get WASM capsule handler registry
    pub fn capsule_registry(&self) -> &Arc<CapsuleRegistry> {
        &self.inner.capsule_registry
    }

    /// Get OpenTelemetry span collector
    pub fn span_collector(&self) -> &Arc<SpanCollector> {
        &self.inner.span_collector
    }

    /// Get custom metrics registry
    pub fn custom_metrics(&self) -> &Arc<CustomMetrics> {
        &self.inner.custom_metrics
    }

    /// Get error tracker
    pub fn error_tracker(&self) -> &Arc<ErrorTracker> {
        &self.inner.error_tracker
    }

    /// Get server profiler
    pub fn profiler(&self) -> &Arc<ServerProfiler> {
        &self.inner.profiler
    }

    /// Get config reloader
    pub fn config_reloader(&self) -> &Arc<ConfigReloader> {
        &self.inner.config_reloader
    }

    /// Get plugin manager
    pub fn plugin_manager(&self) -> &Arc<PluginManager> {
        &self.inner.plugin_manager
    }

    /// Get server name
    pub fn name(&self) -> &str {
        &self.inner.name
    }

    /// Get server version
    pub fn version(&self) -> &str {
        &self.inner.version
    }

    /// Record request start for metrics
    pub fn request_start(&self) {
        self.inner.metrics.request_start();
    }

    /// Record request end with duration for metrics
    pub fn request_end(&self, duration_ms: u64) {
        self.inner.metrics.request_end(duration_ms);
    }

    /// Record an upstream error
    pub fn upstream_error(&self) {
        self.inner.metrics.upstream_error();
    }

    /// Record a route error
    pub fn route_error(&self) {
        self.inner.metrics.route_error();
    }

    /// Sync metrics from runtime counters (for /metrics endpoint)
    pub fn sync_metrics(&self) {
        let counters = self.inner.runtime.counters_snapshot();
        let latency = self.inner.runtime.latency_snapshot();
        self.inner.metrics.sync_from_runtime(&counters, &latency);
    }
}

/// Get the default plugins directory path.
fn dirs_plugin_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    std::path::PathBuf::from(home).join(".vil").join("plugins")
}
