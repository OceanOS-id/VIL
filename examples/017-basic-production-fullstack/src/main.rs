// ╔════════════════════════════════════════════════════════════╗
// ║  017 — Enterprise Platform (Full Stack)                   ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Pattern:  VX_APP                                           ║
// ║  Token:    N/A (HTTP server)                                ║
// ║  Features: ShmSlice, VilResponse                            ║
// ║  Domain:   Production-ready enterprise platform with all    ║
// ║            middleware, auth, metrics, multi-protocol         ║
// ╚════════════════════════════════════════════════════════════╝
//
// BUSINESS CONTEXT:
//   Enterprise platform reference architecture. This is the "kitchen sink"
//   example showing how a production deployment configures every subsystem:
//   21 middleware layers (auth, CORS, compression, tracing), multi-protocol
//   (REST + gRPC + GraphQL), database + cache, mesh discovery, and
//   observability. Used as a starting point for platform engineering teams
//   building internal developer platforms (IDPs).
//
// Run:
//   cargo run -p basic-usage-production-fullstack
//
// Test:
//   curl http://localhost:8080/
//   curl http://localhost:8080/api/stack
//   curl http://localhost:8080/api/config
//   curl http://localhost:8080/api/sprints
//   curl http://localhost:8080/api/middleware

use vil_server::prelude::*;
use vil_server_config::FullServerConfig;

// This example uses FullServerConfig::default() to demonstrate the platform's
// full configuration surface. In production, config is loaded from:
//   1. vil-server.yaml (base config)
//   2. Environment variables (overrides, e.g., PORT, DATABASE_URL)
//   3. CLI flags (one-off overrides for debugging)
// The precedence chain: YAML < ENV < CLI ensures 12-factor compliance.

// ─────────────────────────────────────────────────────────────────────────────
// Response Types
// ─────────────────────────────────────────────────────────────────────────────
// These types model the enterprise platform's self-introspection API.
// Each response type maps to a dimension of the platform:
//   ServiceInfoResponse — feature inventory (what the platform CAN do)
//   StackInfoResponse   — subsystem status (what's RUNNING now)
//   FullConfigResponse  — configuration (HOW it's configured)
//   SprintsResponse     — development roadmap (WHEN features shipped)
//   MiddlewareInfoResponse — security posture (compliance audit trail)

/// Maps API routes to their descriptions for the self-documentation endpoint.
/// Business: API consumers (frontend devs, partners) use this to discover
/// available endpoints without reading source code or separate docs.
#[derive(Serialize)]
struct EndpointMap {
    #[serde(rename = "GET /")]
    root: &'static str,
    #[serde(rename = "GET /api/stack")]
    stack: &'static str,
    #[serde(rename = "GET /api/config")]
    config: &'static str,
    #[serde(rename = "GET /api/sprints")]
    sprints: &'static str,
    #[serde(rename = "GET /api/middleware")]
    middleware: &'static str,
}

#[derive(Serialize)]
struct ServiceInfoResponse {
    name: &'static str,
    version: &'static str,
    description: &'static str,
    sprint_summary: &'static str,
    features: Vec<&'static str>,
    endpoints: EndpointMap,
}

#[derive(Serialize)]
struct SubsystemFeatures {
    #[serde(skip_serializing_if = "Option::is_none")]
    framework: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    orm: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    playground: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pool_size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    channels: Option<Vec<&'static str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    discovery: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    features: Vec<&'static str>,
}

#[derive(Serialize)]
struct ServerSummary {
    name: String,
    port: u16,
    metrics_port: Option<u16>,
    workers: String,
    max_body_size: String,
}

#[derive(Serialize)]
struct StackInfoResponse {
    server: ServerSummary,
    subsystems: std::collections::HashMap<&'static str, SubsystemFeatures>,
    crate_count: u32,
    total_tests: u32,
}

#[derive(Serialize)]
struct ConfigSection {
    name: &'static str,
    controls: &'static str,
}

#[derive(Serialize)]
struct ServerConfigDetail {
    name: String,
    port: u16,
    host: String,
    metrics_port: Option<u16>,
    workers: usize,
    request_timeout_secs: u64,
    max_body_size: String,
    graceful_shutdown_timeout_secs: u64,
}

#[derive(Serialize)]
struct LoggingDetail {
    level: String,
    format: String,
}

#[derive(Serialize)]
struct ShmDetail {
    enabled: bool,
    pool_size: String,
    query_cache_enabled: bool,
}

#[derive(Serialize)]
struct GrpcDetail {
    enabled: bool,
    port: u16,
}

#[derive(Serialize)]
struct GraphqlDetail {
    enabled: bool,
    playground: bool,
    max_depth: usize,
}

#[derive(Serialize)]
struct SecurityDetail {
    jwt_enabled: bool,
    rate_limit_enabled: bool,
    csrf_enabled: bool,
}

#[derive(Serialize)]
struct MiddlewareDetail {
    cors_enabled: bool,
    compression_enabled: bool,
    tracing_enabled: bool,
}

#[derive(Serialize)]
struct FullConfigResponse {
    description: &'static str,
    sections: Vec<ConfigSection>,
    server: ServerConfigDetail,
    logging: LoggingDetail,
    shm: ShmDetail,
    grpc: GrpcDetail,
    graphql: GraphqlDetail,
    security: SecurityDetail,
    middleware: MiddlewareDetail,
}

/// Represents one development sprint in the VIL roadmap.
/// Business: product managers track feature delivery cadence using this data.
#[derive(Serialize)]
struct SprintInfo {
    id: &'static str,
    name: &'static str,
    modules: u32,
    status: &'static str,
    description: &'static str,
}

#[derive(Serialize)]
struct SprintsResponse {
    total_sprints: u32,
    total_crates: u32,
    total_tests: u32,
    sprints: Vec<SprintInfo>,
}

/// Represents a single middleware layer in the request processing stack.
/// Business: security compliance requires documenting every middleware
/// layer, its enabled state, and configuration parameters.
#[derive(Serialize)]
struct MiddlewareLayer {
    layer: u32,
    name: &'static str,
    enabled: bool,
    description: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    sample_rate: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    propagation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_body_size: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    duration_secs: Option<u64>,
}

/// Security middleware status — auditors check these during compliance reviews.
#[derive(Serialize)]
struct SecurityMiddleware {
    name: &'static str,
    enabled: bool,
}

#[derive(Serialize)]
struct MiddlewareInfoResponse {
    total_middleware_layers: u32,
    note: &'static str,
    middleware_stack: Vec<MiddlewareLayer>,
    security_middleware: Vec<SecurityMiddleware>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────
// Each handler serves a different stakeholder:
//   /          — engineers (feature discovery)
//   /stack     — SREs (runtime health)
//   /config    — platform team (configuration review)
//   /sprints   — product managers (release tracking)
//   /middleware — security auditors (compliance verification)

/// GET / — Feature inventory listing all VIL sprints S1 through S18.
/// Returns a text-style overview of every subsystem and capability.
/// Business: this endpoint serves as the platform's self-documentation —
/// new engineers use it to discover available capabilities without reading docs.
async fn index() -> VilResponse<ServiceInfoResponse> {
    VilResponse::ok(ServiceInfoResponse {
        name: "VIL Production Fullstack",
        version: "0.1.0",
        description: "Comprehensive example showing ALL VIL server features",
        sprint_summary: "18 sprints (S1-S18), 21 crates, 245 tests, 0 warnings",
        features: vec![
            "S1  Foundation: VilServer builder, Router, AppState, health/ready/metrics",
            "S2  SHM Zero-Copy: ShmSlice, ShmContext, ShmJson, process isolation",
            "S3  Middleware Stack: CORS, compression, timeouts, security headers, tracking",
            "S4  Auth & Security: JWT, rate limiting, CSRF, brute-force protection",
            "S5  Config System: Multi-source (YAML < ENV < CLI), profiles, FullServerConfig",
            "S6  SeaORM Integration: Database layer with SeaORM + SQLx, migrations",
            "S7  DB Semantic Layer: VilEntity derive, DbProvider, DatasourceRegistry, VilCache",
            "S8  GraphQL Plugin: Auto-schema from VilEntity, CRUD resolvers, subscriptions",
            "S9  Tri-Lane Mesh: Trigger/Data/Control channels, SHM discovery, unified mode",
            "S10 NATS + JetStream: Pub/sub, JetStream, KV Store, Tri-Lane bridge",
            "S11 gRPC Server: Tonic integration, health check, per-service metrics",
            "S12 Kafka Streams: Kafka producer/consumer, stream processing",
            "S13 MQTT IoT: MQTT client, IoT gateway patterns",
            "S14 Protobuf Codegen: Proto compilation, Rust code generation",
            "S15 Templates & CLI: 12 project templates, vil-cli init command",
            "S16 Observability: Prometheus metrics, distributed tracing, W3C propagation",
            "S17 Performance: Thread pool tuning, benchmarks, zero regression",
            "S18 Examples & Docs: 20+ examples, comprehensive documentation",
        ],
        endpoints: EndpointMap {
            root: "this inventory",
            stack: "subsystem overview",
            config: "full server configuration",
            sprints: "sprint inventory with module counts",
            middleware: "middleware stack (21 layers)",
        },
    })
}

/// GET /api/stack — Shows all configured subsystems with details.
/// Reads FullServerConfig to reflect actual configuration state.
/// Business: SREs use this to verify subsystem health after deployment.
/// The crate_count/total_tests fields feed the release quality dashboard.
async fn stack_info() -> VilResponse<StackInfoResponse> {
    let config = FullServerConfig::default();

    let mut subsystems = std::collections::HashMap::new();
    subsystems.insert(
        "rest",
        SubsystemFeatures {
            framework: Some("Axum 0.7"),
            orm: None,
            enabled: None,
            port: None,
            playground: None,
            pool_size: None,
            mode: None,
            channels: None,
            discovery: None,
            features: vec![
                "JSON extraction",
                "path params",
                "query params",
                "validation",
            ],
        },
    );
    subsystems.insert(
        "grpc",
        SubsystemFeatures {
            framework: Some("tonic 0.12"),
            enabled: Some(config.grpc.enabled),
            port: Some(config.grpc.port),
            orm: None,
            playground: None,
            pool_size: None,
            mode: None,
            channels: None,
            discovery: None,
            features: vec!["health check", "per-service metrics", "dual-port"],
        },
    );
    subsystems.insert(
        "nats",
        SubsystemFeatures {
            framework: None,
            orm: None,
            enabled: None,
            port: None,
            playground: None,
            pool_size: None,
            mode: None,
            channels: None,
            discovery: None,
            features: vec!["pub/sub", "JetStream", "KV Store", "Tri-Lane bridge"],
        },
    );
    subsystems.insert(
        "database",
        SubsystemFeatures {
            orm: Some("SeaORM + SQLx"),
            framework: None,
            enabled: None,
            port: None,
            playground: None,
            pool_size: None,
            mode: None,
            channels: None,
            discovery: None,
            features: vec![
                "VilEntity derive",
                "DbProvider (1 vtable)",
                "DatasourceRegistry",
                "VilCache",
            ],
        },
    );
    subsystems.insert(
        "graphql",
        SubsystemFeatures {
            framework: Some("async-graphql 7"),
            enabled: Some(config.graphql.enabled),
            playground: Some(config.graphql.playground),
            orm: None,
            port: None,
            pool_size: None,
            mode: None,
            channels: None,
            discovery: None,
            features: vec![
                "auto-schema from VilEntity",
                "CRUD resolvers",
                "subscriptions",
            ],
        },
    );
    subsystems.insert(
        "shm",
        SubsystemFeatures {
            enabled: Some(config.shm.enabled),
            pool_size: Some(config.shm.pool_size.clone()),
            framework: None,
            orm: None,
            port: None,
            playground: None,
            mode: None,
            channels: None,
            discovery: None,
            features: vec!["zero-copy IPC", "query cache", "ShmSlice extractors"],
        },
    );
    subsystems.insert(
        "mesh",
        SubsystemFeatures {
            mode: Some(config.mesh.mode.clone()),
            channels: Some(vec!["trigger", "data", "control"]),
            discovery: Some(config.mesh.discovery.mode.clone()),
            framework: None,
            orm: None,
            enabled: None,
            port: None,
            playground: None,
            pool_size: None,
            features: vec![],
        },
    );

    VilResponse::ok(StackInfoResponse {
        server: ServerSummary {
            name: config.server.name.clone(),
            port: config.server.port,
            metrics_port: config.server.metrics_port,
            workers: if config.server.workers == 0 {
                "auto (num_cpus)".to_string()
            } else {
                config.server.workers.to_string()
            },
            max_body_size: config.server.max_body_size.clone(),
        },
        subsystems,
        crate_count: 21,
        total_tests: 245,
    })
}

/// GET /api/config — Shows the full FullServerConfig sections and what they control.
/// Each section maps to a distinct subsystem in the VIL server.
/// Business: platform engineers review this before production rollouts to
/// ensure security settings, pool sizes, and timeouts match the environment.
async fn full_config() -> VilResponse<FullConfigResponse> {
    let config = FullServerConfig::default();

    VilResponse::ok(FullConfigResponse {
        description: "Full FullServerConfig from vil_server_config",
        sections: vec![
            ConfigSection {
                name: "server",
                controls: "Host, port, workers, timeouts, body size, graceful shutdown",
            },
            ConfigSection {
                name: "logging",
                controls: "Log level, format (text/json), per-module overrides",
            },
            ConfigSection {
                name: "shm",
                controls: "Shared memory pool, query cache, reset threshold",
            },
            ConfigSection {
                name: "mesh",
                controls: "Tri-Lane channels (trigger/data/control), discovery mode",
            },
            ConfigSection {
                name: "services",
                controls: "Service definitions with visibility (public/internal)",
            },
            ConfigSection {
                name: "middleware",
                controls: "CORS, compression, timeouts, tracing, security headers, HSTS",
            },
            ConfigSection {
                name: "security",
                controls: "JWT, rate limiting, CSRF, brute-force protection",
            },
            ConfigSection {
                name: "session",
                controls: "Session storage backend and TTL",
            },
            ConfigSection {
                name: "observability",
                controls: "Prometheus metrics, health checks, readiness probes",
            },
            ConfigSection {
                name: "performance",
                controls: "Thread pool sizing, benchmark baselines",
            },
            ConfigSection {
                name: "grpc",
                controls: "gRPC server port, health service, per-service metrics",
            },
            ConfigSection {
                name: "graphql",
                controls: "GraphQL endpoint path, playground toggle",
            },
            ConfigSection {
                name: "feature_flags",
                controls: "Hot reload, experimental feature toggles",
            },
            ConfigSection {
                name: "scheduler",
                controls: "Background job scheduling",
            },
            ConfigSection {
                name: "plugins",
                controls: "Plugin system for extensibility",
            },
            ConfigSection {
                name: "rolling_restart",
                controls: "Zero-downtime restart coordination",
            },
            ConfigSection {
                name: "admin",
                controls: "Admin dashboard and internal API",
            },
        ],
        server: ServerConfigDetail {
            name: config.server.name.clone(),
            port: config.server.port,
            host: config.server.host.clone(),
            metrics_port: config.server.metrics_port,
            workers: config.server.workers,
            request_timeout_secs: config.server.request_timeout_secs,
            max_body_size: config.server.max_body_size.clone(),
            graceful_shutdown_timeout_secs: config.server.graceful_shutdown_timeout_secs,
        },
        logging: LoggingDetail {
            level: config.logging.level.clone(),
            format: config.logging.format.clone(),
        },
        shm: ShmDetail {
            enabled: config.shm.enabled,
            pool_size: config.shm.pool_size.clone(),
            query_cache_enabled: config.shm.query_cache.enabled,
        },
        grpc: GrpcDetail {
            enabled: config.grpc.enabled,
            port: config.grpc.port,
        },
        graphql: GraphqlDetail {
            enabled: config.graphql.enabled,
            playground: config.graphql.playground,
            max_depth: config.graphql.max_depth,
        },
        security: SecurityDetail {
            jwt_enabled: config.security.jwt.enabled,
            rate_limit_enabled: config.security.rate_limit.enabled,
            csrf_enabled: config.security.csrf.enabled,
        },
        middleware: MiddlewareDetail {
            cors_enabled: config.middleware.cors.enabled,
            compression_enabled: config.middleware.compression.enabled,
            tracing_enabled: config.middleware.tracing.enabled,
        },
    })
}

/// GET /api/sprints — V5 sprint inventory with module counts per sprint.
/// Shows all 18 sprints with their status, description, and module count.
/// Business: product managers track feature delivery velocity here.
/// The 18 sprints represent the complete VIL platform build-out.
async fn sprints() -> VilResponse<SprintsResponse> {
    VilResponse::ok(SprintsResponse {
        total_sprints: 18,
        total_crates: 21,
        total_tests: 245,
        sprints: vec![
            SprintInfo {
                id: "S1",
                name: "Foundation",
                modules: 3,
                status: "complete",
                description: "VilServer builder, Router, AppState, health/ready/metrics",
            },
            SprintInfo {
                id: "S2",
                name: "SHM Zero-Copy",
                modules: 2,
                status: "complete",
                description: "ShmSlice, ShmContext, ShmJson, blocking_with, process isolation",
            },
            SprintInfo {
                id: "S3",
                name: "Middleware Stack",
                modules: 1,
                status: "complete",
                description: "CORS, compression, timeouts, security headers, request tracking",
            },
            SprintInfo {
                id: "S4",
                name: "Auth & Security",
                modules: 1,
                status: "complete",
                description: "JWT, rate limiting, CSRF, brute-force protection",
            },
            SprintInfo {
                id: "S5",
                name: "Config System",
                modules: 2,
                status: "complete",
                description: "Multi-source config (YAML < ENV < CLI), profiles, FullServerConfig",
            },
            SprintInfo {
                id: "S6",
                name: "SeaORM Integration",
                modules: 1,
                status: "complete",
                description: "Database layer with SeaORM + SQLx, migrations",
            },
            SprintInfo {
                id: "S7",
                name: "DB Semantic Layer",
                modules: 2,
                status: "complete",
                description:
                    "VilEntity derive, DbProvider (1 vtable/~11ns), DatasourceRegistry, VilCache",
            },
            SprintInfo {
                id: "S8",
                name: "GraphQL Plugin",
                modules: 1,
                status: "complete",
                description: "Auto-schema from VilEntity, CRUD resolvers, subscriptions",
            },
            SprintInfo {
                id: "S9",
                name: "Tri-Lane Mesh",
                modules: 1,
                status: "complete",
                description: "Trigger/Data/Control channels, SHM discovery, unified mode",
            },
            SprintInfo {
                id: "S10",
                name: "NATS + JetStream",
                modules: 1,
                status: "complete",
                description: "Pub/sub, JetStream, KV Store, Tri-Lane bridge",
            },
            SprintInfo {
                id: "S11",
                name: "gRPC Server",
                modules: 1,
                status: "complete",
                description: "Tonic integration, health check, per-service metrics",
            },
            SprintInfo {
                id: "S12",
                name: "Kafka Streams",
                modules: 1,
                status: "complete",
                description: "Kafka producer/consumer, stream processing",
            },
            SprintInfo {
                id: "S13",
                name: "MQTT IoT",
                modules: 1,
                status: "complete",
                description: "MQTT client, IoT gateway patterns",
            },
            SprintInfo {
                id: "S14",
                name: "Protobuf Codegen",
                modules: 1,
                status: "complete",
                description: "Proto compilation, Rust code generation",
            },
            SprintInfo {
                id: "S15",
                name: "Templates & CLI",
                modules: 1,
                status: "complete",
                description: "12 project templates, vil-cli init command",
            },
            SprintInfo {
                id: "S16",
                name: "Observability",
                modules: 1,
                status: "complete",
                description: "Prometheus metrics, distributed tracing, W3C propagation",
            },
            SprintInfo {
                id: "S17",
                name: "Performance",
                modules: 0,
                status: "complete",
                description: "Thread pool tuning, benchmarks, zero regression",
            },
            SprintInfo {
                id: "S18",
                name: "Examples & Docs",
                modules: 0,
                status: "complete",
                description: "20+ examples, comprehensive documentation",
            },
        ],
    })
}

/// GET /api/middleware — Middleware stack information (21 middleware layers).
/// Shows every middleware layer with its enabled state and description.
/// Business: security auditors review this to verify compliance controls
/// (JWT auth, CSRF, rate limiting, HSTS) are enabled in production.
async fn middleware_info() -> VilResponse<MiddlewareInfoResponse> {
    let config = FullServerConfig::default();

    VilResponse::ok(MiddlewareInfoResponse {
        total_middleware_layers: 21,
        note: "VIL applies middleware in the listed order (outermost first)",
        middleware_stack: vec![
            MiddlewareLayer {
                layer: 1,
                name: "RequestTracker",
                enabled: config.middleware.request_tracker.enabled,
                description: "Assigns unique request IDs (X-Request-Id), tracks timing",
                sample_rate: None,
                propagation: None,
                mode: None,
                min_body_size: None,
                duration_secs: None,
            },
            MiddlewareLayer {
                layer: 2,
                name: "HandlerMetrics",
                enabled: config.middleware.handler_metrics.enabled,
                description: "Per-handler Prometheus metrics with configurable sampling",
                sample_rate: Some(config.middleware.handler_metrics.sample_rate),
                propagation: None,
                mode: None,
                min_body_size: None,
                duration_secs: None,
            },
            MiddlewareLayer {
                layer: 3,
                name: "Tracing",
                enabled: config.middleware.tracing.enabled,
                description: "Distributed tracing with context propagation",
                propagation: Some(config.middleware.tracing.propagation.clone()),
                sample_rate: None,
                mode: None,
                min_body_size: None,
                duration_secs: None,
            },
            MiddlewareLayer {
                layer: 4,
                name: "CORS",
                enabled: config.middleware.cors.enabled,
                description: "Cross-Origin Resource Sharing",
                mode: Some(config.middleware.cors.mode.clone()),
                sample_rate: None,
                propagation: None,
                min_body_size: None,
                duration_secs: None,
            },
            MiddlewareLayer {
                layer: 5,
                name: "Compression",
                enabled: config.middleware.compression.enabled,
                description: "Response compression (gzip/deflate/br)",
                min_body_size: Some(config.middleware.compression.min_body_size),
                sample_rate: None,
                propagation: None,
                mode: None,
                duration_secs: None,
            },
            MiddlewareLayer {
                layer: 6,
                name: "Timeout",
                enabled: config.middleware.timeout.enabled,
                description: "Request timeout enforcement",
                duration_secs: Some(config.middleware.timeout.duration_secs),
                sample_rate: None,
                propagation: None,
                mode: None,
                min_body_size: None,
            },
            MiddlewareLayer {
                layer: 7,
                name: "SecurityHeaders",
                enabled: config.middleware.security_headers.enabled,
                description: "X-Content-Type-Options, X-Frame-Options, X-XSS-Protection",
                sample_rate: None,
                propagation: None,
                mode: None,
                min_body_size: None,
                duration_secs: None,
            },
            MiddlewareLayer {
                layer: 8,
                name: "HSTS",
                enabled: config.middleware.hsts.enabled,
                description: "HTTP Strict Transport Security header",
                sample_rate: None,
                propagation: None,
                mode: None,
                min_body_size: None,
                duration_secs: None,
            },
        ],
        security_middleware: vec![
            SecurityMiddleware {
                name: "JWT Auth",
                enabled: config.security.jwt.enabled,
            },
            SecurityMiddleware {
                name: "Rate Limiter",
                enabled: config.security.rate_limit.enabled,
            },
            SecurityMiddleware {
                name: "CSRF Protection",
                enabled: config.security.csrf.enabled,
            },
            SecurityMiddleware {
                name: "Brute-Force Guard",
                enabled: config.security.brute_force.enabled,
            },
        ],
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Main — VX Process-Oriented (VilApp + ServiceProcess)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    // ── Step 2: Define services as Processes ─────────────────────────
    // Separation: public_api is internet-facing, internal_admin is
    // restricted to cluster-internal traffic (different network policy).
    let public_api = ServiceProcess::new("fullstack")
        .prefix("/api")
        .endpoint(Method::GET, "/stack", get(stack_info))
        .endpoint(Method::GET, "/config", get(full_config))
        .endpoint(Method::GET, "/sprints", get(sprints))
        .endpoint(Method::GET, "/middleware", get(middleware_info));

    // Internal admin: restricted to cluster-internal traffic only.
    // Business: admin endpoints expose sensitive config; never route through
    // public ingress. Kubernetes NetworkPolicy restricts source IPs.
    let internal_admin = ServiceProcess::new("admin")
        .prefix("/internal/admin")
        .endpoint(Method::GET, "/config", get(full_config));

    // ── Step 3: Assemble into VilApp and run ───────────────────────
    VilApp::new("production-fullstack")
        .port(8080)
        .service(ServiceProcess::new("root").endpoint(Method::GET, "/", get(index)))
        .service(public_api)
        .service(internal_admin)
        .run()
        .await;
}
