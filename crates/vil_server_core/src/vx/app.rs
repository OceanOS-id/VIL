// =============================================================================
// VX VilApp — Process topology builder (Tri-Lane architecture)
// =============================================================================
//
// VilApp is the high-level builder for a VX process-oriented server.
// It composes ServiceProcess definitions into a Tri-Lane mesh and delegates
// the HTTP boundary to the existing VilServer (Phase 1 bridge).
//
// VilServer stays — VilApp is added alongside (backward-compatible).

use std::sync::Arc;

use super::tri_lane::{Lane, TriLaneReceivers, TriLaneRouter};
use vil_shm::ExchangeHeap;

use super::egress::EgressHandle;
use super::ingress::{HttpIngressConfig, IngressBridge};
use super::service::ServiceProcess;
use crate::plugin_system::{PluginInfo, PluginRegistry, VilPlugin};
use crate::router::Visibility;
use crate::server::VilServer;
use vil_sidecar::SidecarConfig;

// =============================================================================
// VxMeshConfig — Tri-Lane mesh routing configuration
// =============================================================================

/// A single mesh route entry.
#[derive(Debug, Clone)]
pub struct MeshRouteEntry {
    /// Source service name
    pub from: String,
    /// Target service name
    pub to: String,
    /// Which lane to use
    pub lane: Lane,
}

/// Backpressure configuration for a service.
#[derive(Debug, Clone)]
pub struct BackpressureEntry {
    /// Service name
    pub service: String,
    /// Maximum in-flight messages
    pub max_in_flight: usize,
}

/// Tri-Lane mesh routing configuration.
///
/// Defines how services communicate via the Tri-Lane router.
/// Lane selection determines the SHM channel used:
/// - Trigger: request init, auth tokens
/// - Data: payload stream (zero-copy SHM)
/// - Control: backpressure, circuit breaker, health
#[derive(Debug, Clone, Default)]
pub struct VxMeshConfig {
    routes: Vec<MeshRouteEntry>,
    backpressure: Vec<BackpressureEntry>,
}

impl VxMeshConfig {
    /// Create a new empty mesh config.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a route between two services on a specific lane.
    ///
    /// The transfer mode is auto-selected based on the lane:
    /// - Trigger/Control: Copy mode (small messages)
    /// - Data: LoanWrite mode (zero-copy SHM)
    pub fn route(mut self, from: impl Into<String>, to: impl Into<String>, lane: Lane) -> Self {
        self.routes.push(MeshRouteEntry {
            from: from.into(),
            to: to.into(),
            lane,
        });
        self
    }

    /// Set backpressure limits for a service.
    pub fn backpressure(mut self, service: impl Into<String>, max_in_flight: usize) -> Self {
        self.backpressure.push(BackpressureEntry {
            service: service.into(),
            max_in_flight,
        });
        self
    }

    /// Get all defined routes.
    pub fn routes(&self) -> &[MeshRouteEntry] {
        &self.routes
    }

    /// Get the number of routes.
    pub fn route_count(&self) -> usize {
        self.routes.len()
    }
}

// =============================================================================
// VxFailoverConfig — Failover settings
// =============================================================================

/// Strategy for failing over from a primary to a backup service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailoverStrategy {
    /// Switch to backup immediately on first failure.
    Immediate,
    /// Retry the primary N times before switching to backup.
    Retry(u32),
}

impl Default for FailoverStrategy {
    fn default() -> Self {
        FailoverStrategy::Retry(3)
    }
}

/// A single failover entry.
#[derive(Debug, Clone)]
pub struct FailoverEntry {
    /// Primary service name
    pub primary: String,
    /// Backup service name
    pub backup: String,
    /// Failover strategy
    pub strategy: FailoverStrategy,
}

/// Failover configuration for the VX topology.
#[derive(Debug, Clone, Default)]
pub struct VxFailoverConfig {
    entries: Vec<FailoverEntry>,
}

impl VxFailoverConfig {
    /// Create a new empty failover config.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a failover pair.
    pub fn backup(
        mut self,
        primary: impl Into<String>,
        backup: impl Into<String>,
        strategy: FailoverStrategy,
    ) -> Self {
        self.entries.push(FailoverEntry {
            primary: primary.into(),
            backup: backup.into(),
            strategy,
        });
        self
    }

    /// Get the failover entries.
    pub fn entries(&self) -> &[FailoverEntry] {
        &self.entries
    }
}

// =============================================================================
// VilApp — Main Process topology builder
// =============================================================================

/// Process topology builder for VIL's Tri-Lane architecture.
///
/// VilApp composes ServiceProcess definitions, configures the Tri-Lane
/// mesh, and delegates to VilServer for the HTTP boundary.
///
/// # Example
/// ```ignore
/// VilApp::new("my-app")
///     .port(8080)
///     .service(
///         ServiceProcess::new("users")
///             .endpoint(Method::GET, "/", get(list_users))
///     )
///     .service(
///         ServiceProcess::new("orders")
///             .endpoint(Method::GET, "/", get(list_orders))
///     )
///     .mesh(
///         VxMeshConfig::new()
///             .route("orders", "users", Lane::Data)
///     )
///     .run()
///     .await;
/// ```
pub struct VilApp {
    /// Application name
    name: String,
    /// HTTP ingress configuration
    ingress: HttpIngressConfig,
    /// Registered service processes
    services: Vec<ServiceProcess>,
    /// Tri-Lane mesh configuration
    mesh_config: Option<VxMeshConfig>,
    /// Failover configuration
    failover_config: Option<VxFailoverConfig>,
    /// Global shared state (accessible by all services)
    global_state: Option<Arc<dyn std::any::Any + Send + Sync>>,
    /// ExchangeHeap size in bytes
    heap_size: usize,
    /// Sidecar configurations for ExecClass::SidecarProcess endpoints.
    sidecar_configs: Vec<SidecarConfig>,
    /// Plugin registry (hybrid architecture: native + process + WASM + sidecar)
    plugin_registry: PluginRegistry,
    /// Enable the embedded observer dashboard at `/_vil/dashboard/`.
    observer: bool,
}

impl VilApp {
    /// Create a new VilApp with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ingress: HttpIngressConfig::default(),
            services: Vec::new(),
            mesh_config: None,
            failover_config: None,
            global_state: None,
            heap_size: 64 * 1024 * 1024, // 64MB default
            sidecar_configs: Vec::new(),
            plugin_registry: PluginRegistry::new(),
            observer: false,
        }
    }

    /// Set the HTTP listening port.
    pub fn port(mut self, port: u16) -> Self {
        self.ingress.port = port;
        self
    }

    /// Ensure the port is free before starting — kills any stale process.
    ///
    /// ```ignore
    /// VilApp::new("my-app")
    ///     .port(3080)
    ///     .ensure_port_free()
    /// ```
    pub fn ensure_port_free(self) -> Self {
        let port = self.ingress.port;
        if std::net::TcpListener::bind(("0.0.0.0", port)).is_err() {
            eprintln!("Port {} in use — releasing...", port);
            #[cfg(unix)]
            {
                let _ = std::process::Command::new("sh")
                    .args(["-c", &format!("kill $(lsof -ti:{}) 2>/dev/null", port)])
                    .output();
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        }
        self
    }

    /// Set a separate metrics/health port.
    pub fn metrics_port(mut self, port: u16) -> Self {
        self.ingress.metrics_port = Some(port);
        self
    }

    /// Disable CORS.
    pub fn no_cors(mut self) -> Self {
        self.ingress.cors = false;
        self
    }

    /// Enable the embedded observer dashboard at `/_vil/dashboard/`.
    pub fn observer(mut self, enabled: bool) -> Self {
        self.observer = enabled;
        self
    }

    /// Set maximum request body size.
    pub fn max_body_size(mut self, size: usize) -> Self {
        self.ingress.max_body_size = size;
        self
    }

    /// Set the ExchangeHeap size in bytes (default: 64MB).
    pub fn heap_size(mut self, size: usize) -> Self {
        self.heap_size = size;
        self
    }

    /// Apply a profile preset (dev/staging/prod).
    /// Adjusts heap_size based on the profile's SHM capacity.
    pub fn profile(mut self, profile: &str) -> Self {
        match profile {
            "dev" | "development" => {
                self.heap_size = 8 * 1024 * 1024;
            }
            "staging" | "stage" => {
                self.heap_size = 64 * 1024 * 1024;
            }
            "prod" | "production" => {
                self.heap_size = 256 * 1024 * 1024;
            }
            _ => {}
        }
        self
    }

    /// Add a service process to the topology.
    pub fn service(mut self, service: ServiceProcess) -> Self {
        self.services.push(service);
        self
    }

    /// Set the Tri-Lane mesh configuration.
    pub fn mesh(mut self, config: VxMeshConfig) -> Self {
        self.mesh_config = Some(config);
        self
    }

    /// Set the failover configuration.
    pub fn failover(mut self, config: VxFailoverConfig) -> Self {
        self.failover_config = Some(config);
        self
    }

    /// Set global shared state accessible by all services.
    pub fn state<T: Send + Sync + 'static>(mut self, state: T) -> Self {
        self.global_state = Some(Arc::new(state));
        self
    }

    /// Register a sidecar for ExecClass::SidecarProcess endpoints.
    pub fn sidecar(mut self, config: SidecarConfig) -> Self {
        self.sidecar_configs.push(config);
        self
    }

    /// Get registered sidecar configs.
    pub fn sidecar_configs(&self) -> &[SidecarConfig] {
        &self.sidecar_configs
    }

    /// Register a plugin (Tier 1: native, compile-time).
    ///
    /// Plugins are resolved in dependency order and initialized when `.run()` is called.
    /// Multiple plugins can share resources via the `PluginContext`.
    ///
    /// ```ignore
    /// VilApp::new("my-app")
    ///     .plugin(LlmPlugin::new().provider("openai", config))
    ///     .plugin(RagPlugin::new().store(QdrantStore::new(url)))
    ///     .run().await;
    /// ```
    pub fn plugin(mut self, plugin: impl VilPlugin) -> Self {
        self.plugin_registry.add(plugin);
        self
    }

    /// Get the number of registered plugins.
    pub fn plugin_count(&self) -> usize {
        self.plugin_registry.count()
    }

    /// List all registered plugins.
    pub fn plugin_list(&self) -> Vec<PluginInfo> {
        self.plugin_registry.list()
    }

    /// Get the application name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the number of registered services.
    pub fn service_count(&self) -> usize {
        self.services.len()
    }

    /// Get the total number of endpoints across all services.
    pub fn total_endpoints(&self) -> usize {
        self.services.iter().map(|s| s.endpoint_count()).sum()
    }

    /// Export the topology as a JSON string.
    ///
    /// Useful for service discovery, documentation, and debugging.
    pub fn contract_json(&self) -> String {
        let services: Vec<serde_json::Value> = self
            .services
            .iter()
            .map(|svc| {
                let endpoints: Vec<serde_json::Value> = svc
                    .endpoints()
                    .iter()
                    .map(|ep| {
                        serde_json::json!({
                            "method": ep.method.as_str(),
                            "path": format!("{}{}", svc.prefix_path(), ep.path),
                            "handler": ep.handler_name,
                            "exec_class": ep.exec_class.to_string(),
                        })
                    })
                    .collect();

                let semantics: Vec<serde_json::Value> = svc
                    .semantic_declarations()
                    .iter()
                    .map(|sd| {
                        serde_json::json!({
                            "type_name": sd.type_name,
                            "kind": format!("{:?}", sd.kind),
                            "lane": format!("{:?}", sd.lane),
                        })
                    })
                    .collect();

                let mut svc_json = serde_json::json!({
                    "name": svc.name(),
                    "prefix": svc.prefix_path(),
                    "visibility": format!("{:?}", svc.visibility_level()),
                    "endpoints": endpoints,
                });
                if !semantics.is_empty() {
                    svc_json.as_object_mut().unwrap().insert(
                        "semantic_declarations".to_string(),
                        serde_json::Value::Array(semantics),
                    );
                }
                svc_json
            })
            .collect();

        let mesh_routes: Vec<serde_json::Value> = self
            .mesh_config
            .as_ref()
            .map(|m| {
                m.routes
                    .iter()
                    .map(|r| {
                        serde_json::json!({
                            "from": r.from,
                            "to": r.to,
                            "lane": format!("{}", r.lane),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let failover: Vec<serde_json::Value> = self
            .failover_config
            .as_ref()
            .map(|f| {
                f.entries
                    .iter()
                    .map(|e| {
                        serde_json::json!({
                            "primary": e.primary,
                            "backup": e.backup,
                            "strategy": format!("{:?}", e.strategy),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let sidecars: Vec<serde_json::Value> = self
            .sidecar_configs
            .iter()
            .map(|sc| {
                serde_json::json!({
                    "name": sc.name,
                    "command": sc.command,
                    "timeout_ms": sc.timeout_ms,
                    "shm_size": sc.shm_size,
                })
            })
            .collect();

        let contract = serde_json::json!({
            "app": self.name,
            "architecture": "VX (Process-Oriented Tri-Lane)",
            "ingress": {
                "port": self.ingress.port,
                "metrics_port": self.ingress.metrics_port,
                "cors": self.ingress.cors,
                "max_body_size": self.ingress.max_body_size,
            },
            "services": services,
            "mesh_routes": mesh_routes,
            "failover": failover,
            "sidecars": sidecars,
            "heap_size_bytes": self.heap_size,
        });

        serde_json::to_string_pretty(&contract).unwrap_or_else(|_| "{}".to_string())
    }

    /// Run the VX application.
    ///
    /// Internally:
    /// 1. Creates an ExchangeHeap for SHM-backed Tri-Lane communication
    /// 2. Creates IngressBridge and EgressHandle
    /// 3. Creates a TriLaneRouter and registers mesh routes
    /// 4. Spawns background receiver tasks for each mesh route
    /// 5. Builds an Axum router from public services (Phase 1 bridge)
    /// 6. Stores IngressBridge + TriLaneRouter as Axum Extension layers
    /// 7. Delegates to VilServer for the HTTP boundary
    /// 8. Prints the VX topology banner
    pub async fn run(mut self) {
        // 0. Resolve and register plugins (dependency order)
        if self.plugin_registry.count() > 0 {
            match self.plugin_registry.resolve_and_register() {
                Ok((plugin_services, plugin_routes)) => {
                    // Add plugin-provided services
                    for svc in plugin_services {
                        self.services.push(svc);
                    }
                    // Add plugin-provided mesh routes
                    if !plugin_routes.is_empty() {
                        let mut mesh = self.mesh_config.take().unwrap_or_default();
                        for (from, to) in plugin_routes {
                            mesh = mesh.route(from, to, Lane::Data);
                        }
                        self.mesh_config = Some(mesh);
                    }
                    {
                        use vil_log::app_log;
                        app_log!(Info, "plugins.registered", { count: self.plugin_registry.count() as u64 });
                    }
                }
                Err(e) => {
                    eprintln!("  FATAL: Plugin resolution failed: {}", e);
                    std::process::exit(1);
                }
            }
        }

        // 1. Create ExchangeHeap
        let heap = Arc::new(ExchangeHeap::new());

        // 2. Create IngressBridge and EgressHandle
        let bridge = IngressBridge::new();
        let egress = EgressHandle::new(bridge.clone());

        // 3. Create TriLaneRouter and register mesh routes
        let tri_lane = Arc::new(TriLaneRouter::new(heap.clone()));

        // Register "ingress" -> service routes for all public services
        let mut ingress_receivers: Vec<(String, TriLaneReceivers)> = Vec::new();
        for svc in &self.services {
            if svc.visibility_level() == Visibility::Public {
                let receivers = tri_lane.register_route("ingress", svc.name());
                ingress_receivers.push((svc.name().to_string(), receivers));
            }
        }

        // Register user-defined mesh routes (inter-service)
        let mut mesh_receivers: Vec<(String, String, TriLaneReceivers)> = Vec::new();
        if let Some(ref mesh) = self.mesh_config {
            let mut registered = std::collections::HashSet::new();
            for route in &mesh.routes {
                let pair = (route.from.clone(), route.to.clone());
                if registered.insert(pair.clone()) {
                    let receivers = tri_lane.register_route(&route.from, &route.to);
                    mesh_receivers.push((route.from.clone(), route.to.clone(), receivers));
                }
            }
        }

        // 4. Spawn background receiver tasks for ingress -> service routes
        for (svc_name, receivers) in ingress_receivers {
            let egress_handle = egress.clone();
            let name = svc_name.clone();
            tokio::spawn(async move {
                Self::ingress_receiver_worker(name, receivers, egress_handle).await;
            });
        }

        // Spawn background receiver tasks for inter-service mesh routes
        for (from, to, receivers) in mesh_receivers {
            tokio::spawn(async move {
                Self::mesh_receiver_worker(from, to, receivers).await;
            });
        }

        // 5. Print VX topology banner (before consuming services)
        self.print_banner(&tri_lane, &bridge);

        // 6. Build VilServer from public services (Phase 1 bridge)
        let mut server = VilServer::new(&self.name).port(self.ingress.port);

        if let Some(mp) = self.ingress.metrics_port {
            server = server.metrics_port(mp);
        }

        if !self.ingress.cors {
            server = server.no_cors();
        }

        if self.observer {
            server = server.observer(true);
        }

        // Add public services as nested routers (owned — applies Extension layers)
        // Each service gets IngressBridge + TriLaneRouter as Extension layers
        for svc in self.services {
            if svc.visibility_level() == Visibility::Public {
                let prefix = svc.prefix_path().to_owned();
                let svc_name = super::ctx::ServiceName(svc.name().to_string());
                let router = svc.build_router_owned();

                // Apply VIL Extension layers so handlers can extract:
                // - ServiceCtx (via TriLaneRouter + ServiceName)
                // - IngressBridge (for Phase 2 rendezvous)
                let router = router
                    .layer(axum::extract::Extension(bridge.clone()))
                    .layer(axum::extract::Extension(tri_lane.clone()))
                    .layer(axum::extract::Extension(svc_name));

                server = server.nest(&prefix, router);
            }
        }

        // 7. Run the server
        server.run().await;
    }

    /// Background task: receives messages from ingress -> service Trigger Lane.
    ///
    /// In Phase 1, this logs incoming messages. In Phase 2, this will
    /// deserialize RequestDescriptor, call the service handler via Tower,
    /// and send the response back via EgressHandle.
    async fn ingress_receiver_worker(
        service_name: String,
        mut receivers: TriLaneReceivers,
        _egress: EgressHandle,
    ) {
        {
            use vil_log::app_log;
            app_log!(Info, "vx.ingress.worker.started", { service: service_name.as_str() });
        }

        loop {
            tokio::select! {
                msg = receivers.trigger.recv() => {
                    match msg {
                        Some(_lane_msg) => {
                            // debug-level: skip vil_log
                        }
                        None => {
                            use vil_log::app_log;
                            app_log!(Info, "vx.ingress.channel.closed", { service: service_name.as_str(), lane: "Trigger" });
                            break;
                        }
                    }
                }
                msg = receivers.data.recv() => {
                    match msg {
                        Some(_lane_msg) => {
                            // debug-level: skip vil_log
                        }
                        None => {
                            use vil_log::app_log;
                            app_log!(Info, "vx.ingress.channel.closed", { service: service_name.as_str(), lane: "Data" });
                            break;
                        }
                    }
                }
                msg = receivers.control.recv() => {
                    match msg {
                        Some(_lane_msg) => {
                            // debug-level: skip vil_log
                        }
                        None => {
                            use vil_log::app_log;
                            app_log!(Info, "vx.ingress.channel.closed", { service: service_name.as_str(), lane: "Control" });
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Background task: receives messages on inter-service mesh routes.
    ///
    /// Logs all messages for observability. In Phase 2 this will feed into
    /// the service's internal processing pipeline.
    async fn mesh_receiver_worker(from: String, to: String, mut receivers: TriLaneReceivers) {
        {
            use vil_log::app_log;
            app_log!(Info, "vx.mesh.worker.started", { from: from.as_str(), to: to.as_str() });
        }

        loop {
            tokio::select! {
                msg = receivers.trigger.recv() => {
                    match msg {
                        Some(_lane_msg) => { /* debug-level: skip vil_log */ }
                        None => break,
                    }
                }
                msg = receivers.data.recv() => {
                    match msg {
                        Some(_lane_msg) => { /* debug-level: skip vil_log */ }
                        None => break,
                    }
                }
                msg = receivers.control.recv() => {
                    match msg {
                        Some(_lane_msg) => { /* debug-level: skip vil_log */ }
                        None => break,
                    }
                }
            }
        }

        {
            use vil_log::app_log;
            app_log!(Info, "vx.mesh.worker.exiting", { from: from.as_str(), to: to.as_str() });
        }
    }

    /// Print the VX topology banner to stdout.
    fn print_banner(&self, tri_lane: &TriLaneRouter, bridge: &IngressBridge) {
        let _ = bridge; // Used in future phases for stats
        println!();
        println!("  ╔══════════════════════════════════════════════════╗");
        println!("  ║  VX — Process-Oriented Server (Tri-Lane)        ║");
        println!("  ╚══════════════════════════════════════════════════╝");
        println!();
        println!("  App:          {}", self.name);
        println!("  Port:         {}", self.ingress.port);
        if let Some(mp) = self.ingress.metrics_port {
            println!("  Metrics:      {}", mp);
        }
        println!("  Heap:         {} MB", self.heap_size / (1024 * 1024));
        println!("  Services:     {}", self.services.len());
        println!("  Endpoints:    {}", self.total_endpoints());
        println!("  Mesh routes:  {}", tri_lane.route_count());
        println!("  Ingress:      IngressBridge (oneshot rendezvous)");
        println!("  Egress:       EgressHandle (bridge-backed)");
        println!();

        // List services
        for svc in &self.services {
            let vis = match svc.visibility_level() {
                Visibility::Public => "PUBLIC",
                Visibility::Internal => "INTERNAL",
            };
            println!(
                "  [{:>8}]  {} ({} endpoints) -> {}",
                vis,
                svc.name(),
                svc.endpoint_count(),
                svc.prefix_path(),
            );

            for ep in svc.endpoints() {
                println!(
                    "              {:>7} {}{}  [{}]",
                    ep.method.as_str(),
                    svc.prefix_path(),
                    ep.path,
                    ep.exec_class,
                );
            }
        }

        // List mesh routes
        if tri_lane.route_count() > 0 {
            println!();
            println!("  Tri-Lane Mesh:");
            for key in tri_lane.route_keys() {
                println!("    {} (Trigger + Data + Control)", key);
            }
        }

        // List failover
        if let Some(ref fo) = self.failover_config {
            if !fo.entries.is_empty() {
                println!();
                println!("  Failover:");
                for entry in &fo.entries {
                    println!(
                        "    {} -> {} [{:?}]",
                        entry.primary, entry.backup, entry.strategy
                    );
                }
            }
        }

        if !self.sidecar_configs.is_empty() {
            println!();
            println!("  Sidecars:");
            for sc in &self.sidecar_configs {
                let cmd = sc.command.as_deref().unwrap_or("(external)");
                println!("    {} [{}]", sc.name, cmd);
            }
        }

        // List plugins
        if self.plugin_registry.count() > 0 {
            println!();
            println!("  Plugins:");
            for info in self.plugin_registry.list() {
                println!("    {} v{} [{}]", info.id, info.version, info.health);
                for cap in &info.capabilities {
                    println!("      - {}", cap);
                }
            }
        }

        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vil_app_builder() {
        let app = VilApp::new("test-app")
            .port(9090)
            .heap_size(32 * 1024 * 1024)
            .service(ServiceProcess::new("svc-a"))
            .service(ServiceProcess::new("svc-b"))
            .mesh(
                VxMeshConfig::new()
                    .route("svc-a", "svc-b", Lane::Data)
                    .backpressure("svc-b", 100),
            )
            .failover(VxFailoverConfig::new().backup(
                "svc-a",
                "svc-b",
                FailoverStrategy::Immediate,
            ));

        assert_eq!(app.name(), "test-app");
        assert_eq!(app.service_count(), 2);
    }

    #[test]
    fn contract_json_export() {
        let app = VilApp::new("json-test")
            .service(ServiceProcess::new("users"))
            .mesh(VxMeshConfig::new().route("users", "orders", Lane::Trigger));

        let json = app.contract_json();
        assert!(json.contains("json-test"));
        assert!(json.contains("users"));
        assert!(json.contains("Trigger"));
    }

    #[test]
    fn failover_config() {
        let fo = VxFailoverConfig::new().backup("primary", "backup", FailoverStrategy::Retry(5));

        assert_eq!(fo.entries().len(), 1);
    }

    #[test]
    fn mesh_config() {
        let mesh = VxMeshConfig::new()
            .route("a", "b", Lane::Data)
            .route("b", "a", Lane::Control)
            .backpressure("b", 50);

        assert_eq!(mesh.route_count(), 2);
    }
}
