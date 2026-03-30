// =============================================================================
// VIL Server — Main server builder and runner
// =============================================================================
//
// VilServer is the primary entry point for building a vil-server application.
// It wraps Axum with VIL features: auto health endpoints, request tracking,
// structured logging, graceful shutdown, and service-aware routing.

use axum::middleware as axum_mw;
use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use vil_log::{app_log, system_log, types::SystemPayload};

use crate::health;
use crate::middleware;
use crate::router::{merge_services, ServiceDef};
use crate::state::AppState;

/// Builder for a VIL server instance.
///
/// # Example (minimal — 5 lines)
/// ```no_run
/// use vil_server_core::*;
///
/// #[tokio::main]
/// async fn main() {
///     VilServer::new("my-app")
///         .port(8080)
///         .route("/", get(|| async { "Hello from vil-server!" }))
///         .run()
///         .await;
/// }
/// ```
pub struct VilServer {
    name: String,
    port: u16,
    metrics_port: Option<u16>,
    app_router: Router<AppState>,
    services: Vec<ServiceDef>,
    nested_prefixes: Vec<String>,
    cors: bool,
    observer: bool,
}

impl VilServer {
    /// Create a new server builder with the given application name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            port: 8080,
            metrics_port: None,
            app_router: Router::new(),
            services: Vec::new(),
            nested_prefixes: Vec::new(),
            cors: false,
            observer: false,
        }
    }

    /// Enable the embedded observer dashboard at `/_vil/dashboard/`.
    ///
    /// When enabled, merges the `vil_observer` router and injects a
    /// `MetricsCollector` as an Axum Extension so that all observer API
    /// endpoints can query live metrics.
    pub fn observer(mut self, enabled: bool) -> Self {
        self.observer = enabled;
        self
    }

    /// Set the listening port (default: 8080).
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set a separate metrics port (default: same as main port).
    /// When set, /health, /ready, /metrics, /info are served on this port instead.
    pub fn metrics_port(mut self, port: u16) -> Self {
        self.metrics_port = Some(port);
        self
    }

    /// Add a route to the server (same as Axum Router::route).
    pub fn route(
        mut self,
        path: &str,
        method_router: axum::routing::MethodRouter<AppState>,
    ) -> Self {
        self.app_router = self.app_router.route(path, method_router);
        self
    }

    /// Nest a sub-router under a path prefix.
    pub fn nest(mut self, path: &str, router: Router<AppState>) -> Self {
        self.nested_prefixes.push(path.to_string());
        self.app_router = self.app_router.nest(path, router);
        self
    }

    /// Register a named service with its own route namespace.
    /// Services get isolated metrics and can communicate via mesh.
    pub fn service(mut self, name: impl Into<String>, router: Router<AppState>) -> Self {
        self.services.push(ServiceDef::new(name, router));
        self
    }

    /// Register a named service with custom prefix and visibility.
    pub fn service_def(mut self, def: ServiceDef) -> Self {
        self.services.push(def);
        self
    }

    /// Merge an existing Axum router (for Axum migration compatibility).
    pub fn merge(mut self, router: Router<AppState>) -> Self {
        self.app_router = self.app_router.merge(router);
        self
    }

    /// Disable CORS headers entirely.
    pub fn no_cors(mut self) -> Self {
        self.cors = false;
        self
    }

    /// Enable permissive CORS (any origin, any method, any header).
    /// Only use for development or public APIs. Disabled by default.
    pub fn cors_permissive(mut self) -> Self {
        self.cors = true;
        self
    }

    /// Build the final Axum application with all middleware and health endpoints.
    fn build(
        self,
    ) -> (
        Router,
        AppState,
        u16,
        Option<u16>,
        Option<(
            Arc<vil_observer::metrics::MetricsCollector>,
            vil_observer::api::UpstreamData,
        )>,
    ) {
        let state = AppState::new(&self.name);

        // Merge service routers
        let service_router = if !self.services.is_empty() {
            merge_services(self.services)
        } else {
            Router::new()
        };

        // Build the main application router
        let mut app = self.app_router.merge(service_router);

        // Health endpoints on main port (unless separate metrics port)
        if self.metrics_port.is_none() {
            app = app.merge(health::health_router());
        }

        // Admin endpoints (capsule, diagnostics, reload, playground, plugins)
        app = app
            .merge(crate::capsule_handler::capsule_admin_router())
            .merge(crate::diagnostics::diagnostics_router())
            .merge(crate::hot_reload::reload_router())
            .merge(crate::playground::playground_router())
            .merge(crate::plugin_api::plugin_router())
            .merge(crate::plugin_detail_gui::plugin_detail_router());

        // Observer dashboard + API (when enabled)
        let upstream_data = vil_observer::api::UpstreamData::default();
        let observer_collector = if self.observer {
            crate::upstream_metrics::enable();
            let collector = Arc::new(vil_observer::metrics::MetricsCollector::new());
            let obs_router: Router<AppState> = vil_observer::observer_router()
                .layer(axum::Extension(collector.clone()))
                .layer(axum::Extension(upstream_data.clone()))
                .with_state(());
            app = app.merge(obs_router);
            Some((collector, upstream_data))
        } else {
            None
        };

        // Add VIL middleware stack (order: outermost layer runs first)
        app = app.layer(axum_mw::from_fn_with_state(
            state.clone(),
            crate::trace_middleware::tracing_middleware,
        ));

        // handler_metrics only when observer is enabled — true zero overhead when OFF
        if self.observer {
            app = app.layer(axum_mw::from_fn_with_state(
                state.clone(),
                crate::obs_middleware::handler_metrics,
            ));
        }

        app = app
            .layer(axum_mw::from_fn_with_state(
                state.clone(),
                middleware::request_tracker,
            ))
            .layer(TraceLayer::new_for_http());

        // CORS
        if self.cors {
            app = app.layer(CorsLayer::permissive());
        }

        // Attach state
        let app = app.with_state(state.clone());

        (app, state, self.port, self.metrics_port, observer_collector)
    }

    /// Run the server (blocking).
    /// Automatically sets up:
    /// - Structured logging (tracing)
    /// - Health/ready/metrics endpoints
    /// - Request ID propagation
    /// - Request metrics tracking
    /// - Graceful shutdown on SIGTERM/SIGINT
    pub async fn run(self) {
        // Initialize tracing
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "info,tower_http=info".into()),
            )
            .with_target(false)
            .init();

        let name = self.name.clone();
        let observer_enabled = self.observer;
        let first_prefix = self.nested_prefixes.first().cloned()
            .or_else(|| self.services.first().map(|s| format!("/api/{}", s.name)))
            .unwrap_or_else(|| "".into());
        let (app, state, port, metrics_port, observer_collector) = self.build();

        let addr = SocketAddr::from(([0, 0, 0, 0], port));

        system_log!(
            Info,
            SystemPayload {
                event_type: 4, // startup
                ..Default::default()
            }
        );
        app_log!(Info, "server.starting", { name: name.as_str(), port: port as u64 });

        println!();
        println!("  vil-server: {}", name);
        println!("  Listening:    http://0.0.0.0:{}", port);
        println!(
            "  Health:       http://localhost:{}/health",
            metrics_port.unwrap_or(port)
        );
        println!(
            "  Readiness:    http://localhost:{}/ready",
            metrics_port.unwrap_or(port)
        );
        println!(
            "  Metrics:      http://localhost:{}/metrics",
            metrics_port.unwrap_or(port)
        );
        println!(
            "  Info:         http://localhost:{}/info",
            metrics_port.unwrap_or(port)
        );
        if observer_enabled {
            println!("  Observer:     http://localhost:{}/_vil/dashboard/", port);
        }

        // Spawn observer bridge: sync HandlerMetricsRegistry + UpstreamRegistry → observer every 2s
        if let Some((obs, upstream_data)) = observer_collector {
            let hmr = state.handler_metrics().clone();
            tokio::spawn(async move {
                loop {
                    // Sync handler metrics → observer
                    hmr.sync_to_observer(&obs);

                    // Sync upstream metrics → observer
                    let snapshots = crate::upstream_metrics::global().all_snapshots();
                    let json_snapshots: Vec<serde_json::Value> = snapshots
                        .iter()
                        .map(|s| serde_json::to_value(s).unwrap_or_default())
                        .collect();
                    *upstream_data.0.lock().unwrap() = json_snapshots;

                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            });
        }
        println!();
        println!("  Press Ctrl+C to stop");
        println!();

        // Show usage guide with actual service endpoint
        let example_path = format!("{}/trigger", first_prefix);

        println!("  ─── Open another terminal ─────────────────────────────────");
        println!();
        let mut step = 1;
        if observer_enabled {
            println!("  {}. Dashboard:  http://localhost:{}/_vil/dashboard/", step, port);
            println!();
            step += 1;
        }
        println!("  {}. Test:", step);
        println!("     curl -s -X POST http://localhost:{}{} \\", port, example_path);
        println!("       -H 'Content-Type: application/json' \\");
        println!("       -d '{{\"prompt\":\"hello\"}}'");
        step += 1;
        println!();
        println!("  {}. Warmup (required before benchmark):", step);
        println!("     hey -m POST -H 'Content-Type: application/json' \\");
        println!("       -d '{{\"prompt\":\"bench\"}}' -c 300 -n 3000 \\");
        println!("       http://localhost:{}{} > /dev/null", port, example_path);
        step += 1;
        println!();
        println!("  {}. Upstream baseline:", step);
        println!("     hey -m POST -H 'Content-Type: application/json' \\");
        println!("       -d '{{\"model\":\"gpt-4\",\"messages\":[{{\"role\":\"user\",\"content\":\"bench\"}}]}}' \\");
        println!("       -c 300 -n 3000 http://localhost:4545/v1/chat/completions");
        step += 1;
        println!();
        println!("  {}. Gateway benchmark:", step);
        println!("     hey -m POST -H 'Content-Type: application/json' \\");
        println!("       -d '{{\"prompt\":\"bench\"}}' -c 300 -n 3000 \\");
        println!("       http://localhost:{}{}", port, example_path);
        println!();
        let us = step - 1;
        println!("     Compare Requests/sec between step {} vs {} to measure overhead.", us, step);
        println!("     For dashboard view, use: -z 30s instead of -n 3000");
        println!();
        println!("  Alternative: vastar (faster, with SLO insight):");
        println!("     vastar -m POST -H 'Content-Type: application/json' \\");
        println!("       -d '{{\"prompt\":\"bench\"}}' -c 300 -n 3000 \\");
        println!("       http://localhost:{}{}", port, example_path);
        println!();
        println!("  ────────────────────────────────────────────────────────────");
        println!();

        // Start metrics server on separate port if configured
        if let Some(mp) = metrics_port {
            let metrics_state = state.clone();
            tokio::spawn(async move {
                let metrics_app = health::health_router().with_state(metrics_state);
                let metrics_addr = SocketAddr::from(([0, 0, 0, 0], mp));
                app_log!(Info, "metrics.server.starting", { port: mp as u64 });
                let listener = match tokio::net::TcpListener::bind(metrics_addr).await {
                    Ok(l) => l,
                    Err(e) => {
                        app_log!(Warn, "metrics.bind.failed", { port: mp as u64, error: e.to_string() });
                        return;
                    }
                };
                axum::serve(listener, metrics_app)
                    .with_graceful_shutdown(crate::shutdown::shutdown_signal())
                    .await
                    .expect("Metrics server error");
            });
        }

        // Ensure port is free — auto-kill stale process
        {
            let port = addr.port();
            if std::net::TcpListener::bind(addr).is_err() {
                eprintln!("Port {} in use — releasing...", port);
                #[cfg(unix)]
                {
                    let _ = std::process::Command::new("sh")
                        .args(["-c", &format!("kill $(lsof -ti:{}) 2>/dev/null", port)])
                        .output();
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
            }
        }

        // Start main server
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .expect("Failed to bind port");

        axum::serve(listener, app)
            .with_graceful_shutdown(crate::shutdown::shutdown_signal())
            .await
            .expect("Server error");

        system_log!(
            Info,
            SystemPayload {
                event_type: 5, // shutdown
                ..Default::default()
            }
        );
        app_log!(Info, "server.shutdown", {});
    }
}
