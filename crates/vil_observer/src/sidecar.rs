// =============================================================================
// VIL Observer Sidecar — standalone observer for SDK pipelines
// =============================================================================
//
// Spawns a lightweight HTTP server on a side port to serve the observer
// dashboard and API. Attaches to any VIL process via callback functions
// that provide runtime metrics.
//
// Usage:
//   vil_observer::sidecar(9090)
//       .runtime_metrics(move || { /* return JSON snapshot */ })
//       .spawn();

use axum::routing::get;
use axum::Json;
use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::api::UpstreamData;
use crate::metrics::MetricsCollector;

type MetricsFn = Arc<dyn Fn() -> serde_json::Value + Send + Sync>;

/// Sidecar observer builder.
pub struct SidecarBuilder {
    port: u16,
    runtime_fn: Option<MetricsFn>,
    processes_fn: Option<MetricsFn>,
    counters_fn: Option<MetricsFn>,
    upstreams_fn: Option<MetricsFn>,
}

/// Create a sidecar observer builder on the given port.
pub fn sidecar(port: u16) -> SidecarBuilder {
    SidecarBuilder {
        port,
        runtime_fn: None,
        processes_fn: None,
        counters_fn: None,
        upstreams_fn: None,
    }
}

impl SidecarBuilder {
    /// Attach to a VastarRuntimeWorld — auto-wires all runtime metrics.
    ///
    /// ```ignore
    /// vil_observer::sidecar(3180).attach(&world).spawn();
    /// ```
    #[cfg(feature = "runtime")]
    pub fn attach(self, world: &Arc<vil_rt::VastarRuntimeWorld>) -> Self {
        let w1 = world.clone();
        let w2 = world.clone();
        let w3 = world.clone();
        self.runtime_metrics(move || {
            let m = w1.metrics_snapshot();
            let inbound = vil_new_http::sink::inbound_snapshot();
            serde_json::json!({
                "queue_depth_total": m.queue_depth_total,
                "in_flight_samples": m.in_flight_samples,
                "registered_processes": m.registered_processes,
                "inbound_requests": inbound.requests,
                "inbound_completed": inbound.completed,
                "inbound_in_flight": inbound.in_flight,
                "inbound_errors": inbound.errors,
                "inbound_avg_latency_ns": inbound.avg_latency_ns,
                "inbound_min_latency_ns": inbound.min_latency_ns,
                "inbound_max_latency_ns": inbound.max_latency_ns,
                "inbound_p95_ns": inbound.p95_ns,
                "inbound_p99_ns": inbound.p99_ns,
                "inbound_p999_ns": inbound.p999_ns,
            })
        })
        .counters(move || {
            let c = w2.raw_counters().snapshot();
            serde_json::to_value(&c).unwrap_or_default()
        })
        .processes(move || {
            let procs = w3.registry_processes();
            let list: Vec<serde_json::Value> = procs
                .iter()
                .map(|p| serde_json::json!({ "info": format!("{:?}", p) }))
                .collect();
            serde_json::json!(list)
        })
        // Upstream tracking not available in SDK pipeline mode
        // (HttpSource makes direct calls, not via SseCollect)
        .upstreams(|| serde_json::json!([]))
    }

    /// Provide runtime metrics (queue depth, in-flight samples, registered processes).
    pub fn runtime_metrics<F>(mut self, f: F) -> Self
    where
        F: Fn() -> serde_json::Value + Send + Sync + 'static,
    {
        self.runtime_fn = Some(Arc::new(f));
        self
    }

    /// Provide process registry snapshot.
    pub fn processes<F>(mut self, f: F) -> Self
    where
        F: Fn() -> serde_json::Value + Send + Sync + 'static,
    {
        self.processes_fn = Some(Arc::new(f));
        self
    }

    /// Provide raw counters (publishes, receives, drops, crashes, etc.).
    pub fn counters<F>(mut self, f: F) -> Self
    where
        F: Fn() -> serde_json::Value + Send + Sync + 'static,
    {
        self.counters_fn = Some(Arc::new(f));
        self
    }

    /// Provide upstream metrics snapshot.
    pub fn upstreams<F>(mut self, f: F) -> Self
    where
        F: Fn() -> serde_json::Value + Send + Sync + 'static,
    {
        self.upstreams_fn = Some(Arc::new(f));
        self
    }

    /// Spawn the sidecar observer server in a background thread.
    /// Returns immediately — the server runs until the process exits.
    pub fn spawn(self) {
        let port = self.port;
        let collector = Arc::new(MetricsCollector::new());
        collector.init_uptime(); // start uptime clock
        let state = SidecarState {
            collector,
            upstream_data: UpstreamData::default(),
            runtime_fn: self
                .runtime_fn
                .unwrap_or_else(|| Arc::new(|| serde_json::json!({}))),
            processes_fn: self
                .processes_fn
                .unwrap_or_else(|| Arc::new(|| serde_json::json!([]))),
            counters_fn: self
                .counters_fn
                .unwrap_or_else(|| Arc::new(|| serde_json::json!({}))),
            upstreams_fn: self
                .upstreams_fn
                .unwrap_or_else(|| Arc::new(|| serde_json::json!([]))),
        };

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create sidecar tokio runtime");

            rt.block_on(async move {
                let shared = Arc::new(state);

                // Pipeline-specific API routes
                let pipeline_api = {
                    let s = shared.clone();
                    Router::new().route(
                        "/_vil/api/pipeline",
                        get(move || {
                            let s = s.clone();
                            async move { Json((s.runtime_fn)()) }
                        }),
                    )
                };

                let processes_api = {
                    let s = shared.clone();
                    Router::new().route(
                        "/_vil/api/processes",
                        get(move || {
                            let s = s.clone();
                            async move { Json((s.processes_fn)()) }
                        }),
                    )
                };

                let counters_api = {
                    let s = shared.clone();
                    Router::new().route(
                        "/_vil/api/counters",
                        get(move || {
                            let s = s.clone();
                            async move { Json((s.counters_fn)()) }
                        }),
                    )
                };

                // Merge standard observer routes + pipeline-specific
                let app = crate::observer_router()
                    .merge(pipeline_api)
                    .merge(processes_api)
                    .merge(counters_api)
                    .layer(axum::Extension(shared.collector.clone()))
                    .layer(axum::Extension(shared.upstream_data.clone()));

                // Sync upstream data periodically from callback
                {
                    let upstream_data = shared.upstream_data.clone();
                    let upstreams_fn = shared.upstreams_fn.clone();
                    tokio::spawn(async move {
                        loop {
                            let val = (upstreams_fn)();
                            if let serde_json::Value::Array(arr) = val {
                                *upstream_data.0.lock().unwrap() = arr;
                            }
                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        }
                    });
                }

                let addr = SocketAddr::from(([0, 0, 0, 0], port));
                println!(
                    "  Observer sidecar: http://localhost:{}/_vil/dashboard/",
                    port
                );

                let listener = tokio::net::TcpListener::bind(addr)
                    .await
                    .expect("Failed to bind observer sidecar port");
                axum::serve(listener, app).await.ok();
            });
        });
    }
}

struct SidecarState {
    collector: Arc<MetricsCollector>,
    upstream_data: UpstreamData,
    runtime_fn: MetricsFn,
    processes_fn: MetricsFn,
    counters_fn: MetricsFn,
    upstreams_fn: MetricsFn,
}
