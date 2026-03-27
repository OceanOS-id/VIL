# vil-server Developer Guide

**Version:** 5.2.0
**Framework:** Axum + VIL Runtime
**Repository:** https://github.com/OceanOS-id/VIL
**Crates:** 45 Rust + 4 SDK packages | **Tests:** 333+ passing | **Warnings:** 0
**Protocols:** REST, SSE, WebSocket, gRPC, Kafka, MQTT, NATS, Tri-Lane SHM
**Performance:** 2.0M req/s, 860K ShmSlice, <1µs mesh

---

## Quick Start

### Hello World (VilApp + ServiceProcess)

```rust
use vil_server::prelude::*;

#[tokio::main]
async fn main() {
    let hello = ServiceProcess::new("hello")
        .endpoint(Method::GET, "/", get(|| async { "Hello from vil-server!" }));

    VilApp::new("hello")
        .port(8080)
        .service(hello)
        .run()
        .await;
}
```

### Generate a Project

```bash
vil server new my-app                    # hello template
vil server new my-api --template crud    # CRUD template
vil server new my-svc --template multiservice  # multi-service
```

---

## Architecture

VIL introduces a process-oriented architecture where services are modeled as **Processes** communicating over a Tri-Lane SHM mesh.

| Type | Role |
|------|------|
| `VilApp` | Process topology builder — register services, configure mesh, run |
| `ServiceProcess` | Service-as-Process with endpoint registration, visibility, prefix |
| `ServiceCtx` | Process-aware context: `.state::<T>()` (private heap), `.send()` / `.trigger()` / `.control()` (Tri-Lane) |
| `VxMeshConfig` | Inter-service Tri-Lane routing with `.route(from, to, lane)` and `.backpressure()` |
| `VxLane` | Lane selection: `Trigger`, `Data`, `Control` |

```rust
use vil_server::prelude::*;

let auth = ServiceProcess::new("auth")
    .endpoint(Method::GET, "/verify", get(verify_token));

let orders = ServiceProcess::new("orders")
    .prefix("/api")
    .endpoint(Method::POST, "/orders", post(create_order));

let mesh = VxMeshConfig::new()
    .route("orders", "auth", VxLane::Trigger);

VilApp::new("platform")
    .port(8080)
    .service(auth)
    .service(orders)
    .mesh(mesh)
    .run()
    .await;
```

For full details (ExecClass, failover), see the [VIL Developer Guide](../vil/VIL-Developer-Guide.md). VIL also supports deployment to vflow-server for hot-reload scenarios.

> **Legacy note:** `VilServer` and `ServiceDef` remain available for backward compatibility but new projects should use `VilApp` + `ServiceProcess`.

---

## Core Concepts

### 1. VilServer Builder (Legacy)

```rust
VilServer::new("my-app")
    .port(8080)                    // HTTP port
    .metrics_port(9090)            // Separate metrics port
    .route("/path", get(handler))  // Add routes
    .service_def(my_service())     // Add named services
    .no_cors()                     // Disable CORS
    .run()                         // Start server
    .await;
```

**Auto-registered endpoints:**
- `GET /health` — Kubernetes liveness probe
- `GET /ready` — Kubernetes readiness probe
- `GET /metrics` — Prometheus metrics (per-handler auto-instrumented)
- `GET /info` — Server info (version, uptime, SHM regions, handler count)
- `POST /admin/reload/:name` — Hot-reload WASM capsule handler
- `GET /admin/capsules` — List loaded capsule handlers

### 2. Extractors

| Extractor | Description |
|-----------|-------------|
| `Json<T>` | Deserialize JSON request body |
| `Path<T>` | Extract path parameters (`/users/:id`) |
| `Query<T>` | Extract query string parameters |
| `State<AppState>` | Access shared application state |
| `RequestId` | Auto-generated X-Request-Id |
| `ShmSlice` | Zero-copy request body via SHM |
| `ShmContext` | SHM region info and ExchangeHeap access |
| `Valid<T>` | Auto-validate request body |
| `WebSocketUpgrade` | WebSocket upgrade |

### 3. Zero-Copy SHM

```rust
// Request body written to ExchangeHeap — zero additional copy
async fn ingest(body: ShmSlice) -> Json<Status> {
    let bytes = body.as_bytes();     // Direct SHM pointer
    let region = body.region_id();   // For mesh forwarding
    let parsed: MyData = body.json()?; // Deserialize from SHM
    // ...
}
```

### 4. Sync Handlers (CPU-bound)

```rust
// CPU-bound work auto-dispatched to blocking thread pool
async fn predict(body: Bytes) -> impl IntoResponse {
    blocking_with(move || {
        let result = heavy_computation(&body);
        Json(result)
    }).await
}
```

### 5. Multi-Service (Process Monolith)

```rust
VilServer::new("platform")
    .service_def(
        ServiceDef::new("orders", order_routes())
            .prefix("/api")
            .visibility(Visibility::Public)
    )
    .service_def(
        ServiceDef::new("payments", payment_routes())
            .prefix("/internal")
            .visibility(Visibility::Internal)
    )
    .run().await;
```

- **Public** services are exposed on the HTTP port
- **Internal** services are accessible only via the Tri-Lane mesh

---

## Tri-Lane Service Mesh

### Architecture

```
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│  Service A   │    │  Service B   │    │  Service C   │
└──────┬───────┘    └──────┬───────┘    └──────┬───────┘
       │                   │                   │
  ═════╪═══════════════════╪═══════════════════╪═════
       │    Trigger Lane (request init)        │
       │    Data Lane (payloads, zero-copy)    │
       │    Control Lane (backpressure)        │
  ═════╪═══════════════════╪═══════════════════╪═════
       │         /dev/shm/vil_mesh_*         │
```

### Key Advantage
Control Lane is **physically separate** from Data Lane. Backpressure signals (Throttle, Pause, Resume) are never blocked by data congestion.

### YAML Configuration

```yaml
# vil-server.yaml
server:
  name: my-platform
  port: 8080
  metrics_port: 9090
services:
  - name: auth
    visibility: public
    prefix: /auth
  - name: orders
    visibility: public
    prefix: /api
  - name: payments
    visibility: internal
mesh:
  mode: unified
  routes:
    - from: auth
      to: orders
      lane: trigger
    - from: orders
      to: payments
      lane: data
```

---

## Configuration

### Precedence

```
Code Default → YAML file → Profile preset → Environment variables
```

Environment variables always win. Profiles apply tuned defaults per environment.

### Profiles

Three built-in profiles control SHM, logging, database, security, and admin settings:

| Profile | SHM | Logging | DB Pool | Admin | Security |
|---------|-----|---------|---------|-------|----------|
| `dev` | 8MB, check/64 | debug, text | 5 conn | all enabled | off |
| `staging` | 64MB, check/256 | info, json | 20 conn | selective | rate limit on |
| `prod` | 256MB, check/1024 | warn, json | 50 conn | all disabled | hardened |

```rust
// In code
VilApp::new("my-app")
    .profile("prod")    // Apply production tuning
    .port(8080)
    .service(svc)
    .run().await;
```

```bash
# Via environment
VIL_PROFILE=prod cargo run
```

### Full YAML Configuration

See `vil-server.reference.yaml` for all options. Key sections:

```yaml
profile: prod                              # VIL_PROFILE

server:
  port: 8080                               # VIL_SERVER_PORT
  host: "0.0.0.0"                          # VIL_SERVER_HOST
  workers: 0                               # VIL_WORKERS (0 = num_cpus)

shm:
  pool_size: "256MB"                       # VIL_SHM_POOL_SIZE
  reset_threshold_pct: 90                  # VIL_SHM_RESET_PCT
  check_interval: 1024                     # VIL_SHM_CHECK_INTERVAL

pipeline:
  queue_capacity: 4096                     # VIL_PIPELINE_QUEUE_CAPACITY
  session_timeout_secs: 600               # VIL_PIPELINE_SESSION_TIMEOUT

database:
  postgres:
    url: "postgres://vil:vil@db:5432/vil"  # VIL_DATABASE_URL
    max_connections: 50                    # VIL_DATABASE_MAX_CONNECTIONS
  redis:
    url: "redis://redis:6380"              # VIL_REDIS_URL

mq:
  nats:
    url: "nats://nats:4222"                # VIL_NATS_URL
  kafka:
    brokers: "kafka:9092"                  # VIL_KAFKA_BROKERS
  mqtt:
    host: mqtt                             # VIL_MQTT_HOST
    port: 1883                             # VIL_MQTT_PORT
```

### Loading

```rust
use vil_server_config::FullServerConfig;

// From file with profile + env overrides
let config = FullServerConfig::from_file_with_env("vil-server.yaml".as_ref())?;

// From environment only (no file)
let config = FullServerConfig::default();  // then apply_env_overrides()
```

---

## Observability

### Zero-Instrumentation Metrics

Every handler **automatically** generates Prometheus metrics:

```
vil_handler_requests_total{method="GET",route="/api/orders"} 1542
vil_handler_errors_total{method="GET",route="/api/orders"} 3
vil_handler_duration_ms_sum{method="GET",route="/api/orders"} 4521
vil_handler_in_flight{method="GET",route="/api/orders"} 7
```

No `@Timed` annotation. No manual instrumentation. Automatic.

### Health Endpoints

| Endpoint | Purpose |
|----------|---------|
| `GET /health` | Liveness probe (always 200 if running) |
| `GET /ready` | Readiness probe (includes uptime) |
| `GET /metrics` | Prometheus text format |
| `GET /info` | Server metadata, SHM stats, handler count |

---

## Security

### JWT Authentication

```rust
let auth = JwtAuth::new("secret-key");
// Apply as middleware layer
```

### Rate Limiting

```rust
let limiter = RateLimit::new(100, Duration::from_secs(60)); // 100 req/min
if limiter.check("client-ip").is_err() {
    return StatusCode::TOO_MANY_REQUESTS;
}
```

### Circuit Breaker

```rust
let cb = CircuitBreaker::new("upstream-api", CircuitBreakerConfig {
    failure_threshold: 5,
    cooldown: Duration::from_secs(30),
    ..Default::default()
});

// Before calling upstream:
cb.check()?;        // Err if circuit is Open
call_upstream().await;
cb.record_success(); // or cb.record_failure()
```

States: `Closed` → `Open` → `HalfOpen` → `Closed`

---

## WebSocket

```rust
use vil_server::core::websocket::*;

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(|mut socket| async move {
        while let Some(Ok(Message::Text(text))) = socket.recv().await {
            socket.send(Message::Text(format!("echo: {}", text))).await.ok();
        }
    })
}
```

### Typed WebSocket Events (VilWsEvent)

```rust
use vil_server::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize, VilWsEvent)]
#[ws_event(topic = "chat.message")]
struct ChatMessage {
    from: String,
    message: String,
}

// Generated methods:
let topic = ChatMessage::ws_topic();    // "chat.message"
let json = msg.to_ws_json();           // JSON string
msg.broadcast(&ws_hub);                 // Broadcast to all subscribers on topic

// WsHub: topic-based broadcast
let hub = WsHub::new();
let mut rx = hub.subscribe("chat.message");
hub.broadcast("chat.message", json_str);
```

**See:** [`009-basic-usage-websocket-chat`](../examples/009-basic-usage-websocket-chat/src/main.rs)

---

## WASM Hot-Reload

```rust
// Load a WASM handler at startup
state.capsule_registry().load_from_file("plugin", "plugin.wasm")?;

// Hot-reload at runtime (no server restart):
curl -X POST http://localhost:8080/admin/reload/plugin
// → {"handler":"plugin","status":"reloaded","reload_time_us":4200}
```

---

## Testing

```rust
use vil_server_test::TestClient;

#[tokio::test]
async fn test_hello() {
    let app = build_app();
    let client = TestClient::new(app);

    let resp = client.get("/").await;
    resp.assert_ok();
    assert!(resp.text().contains("Hello"));
}
```

### Benchmarking

```rust
use vil_server_test::bench::BenchRunner;

let report = BenchRunner::new(app)
    .requests(10000)
    .concurrency(100)
    .path("/api/orders")
    .run()
    .await;

println!("{}", report);
// Throughput: 85000 req/s
// p99 latency: 120µs
```

---

## SSE (Server-Sent Events)

```rust
use vil_server::core::sse::*;

async fn events() -> impl IntoResponse {
    let stream = async_stream::stream! {
        for i in 0..10 {
            yield SseEvent::json(&serde_json::json!({"count": i}));
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    };
    sse_stream(stream)
}
```

---

## Configuration

### Precedence (highest first)
1. Command-line flags
2. Environment variables (`VIL_SERVER_PORT`, `VIL_LOG_LEVEL`, etc.)
3. Profile-specific file (`vil-server.prod.yaml`)
4. Default config file (`vil-server.yaml`)
5. Built-in defaults

### Profiles
- `dev` — debug logging, 1 worker
- `staging` — info logging
- `prod` — warn logging, max workers

---

## CLI Commands

```bash
vil server new <name>              # Scaffold new project
vil server new <name> -t crud      # CRUD template
vil server new <name> -t multiservice  # Multi-service template
vil server dev                     # Dev mode with auto-restart
vil server dev -p 3000             # Dev mode on custom port
vil run                            # Run pipeline
vil explain E-VIL-SEMANTIC-LANE-01   # Explain error code
vil validate pipeline.yaml         # Validate YAML pipeline
```

---

## Migration from Axum

vil-server is built on Axum. Existing Axum handlers work unchanged:

```rust
// Step 1: Pure Axum handler (works as-is)
async fn hello() -> &'static str { "Hello" }

// Step 2: Add SHM extractor (opt-in zero-copy)
async fn ingest(body: ShmSlice) -> Json<Status> { ... }

// Step 3: Use full VIL features
async fn process(
    shm: ShmContext,
    body: ShmSlice,
    State(state): State<AppState>,
) -> impl IntoResponse { ... }
```

---

## AI Plugin System

### Registering Plugins

```rust
VilApp::new("ai-service")
    .port(8080)
    .plugin(LlmPlugin::new().openai(config))
    .plugin(RagPlugin::new())
    .plugin(AgentPlugin::new().tool(Arc::new(CalculatorTool)))
    .run().await;
```

Plugins resolve dependencies automatically (Kahn's topological sort).

### SseCollect for Upstream Proxying

For single-proxy handlers (AI gateway, LLM chat):

```rust
async fn handler(Json(req): Json<Req>) -> HandlerResult<VilResponse<Resp>> {
    let content = SseCollect::post_to("http://api.openai.com/v1/chat/completions")
        .dialect(SseDialect::openai())
        .bearer_token(std::env::var("OPENAI_API_KEY").unwrap_or_default())
        .body(serde_json::json!({
            "model": "gpt-4", "messages": [{"role": "user", "content": req.prompt}],
            "stream": true
        }))
        .collect_text().await
        .map_err(|e| VilError::internal(e.to_string()))?;
    Ok(VilResponse::ok(Resp { content }))
}
```

### SSE Dialects

| Dialect | Done | json_tap | Auth |
|---------|------|----------|------|
| `openai()` | `data: [DONE]` | `choices[0].delta.content` | `.bearer_token()` |
| `anthropic()` | `event: message_stop` | `delta.text` | `.anthropic_key()` |
| `ollama()` | `"done": true` | `message.content` | None |
| `cohere()` | `event: message-end` | `text` | `.bearer_token()` |
| `gemini()` | TCP EOF | `candidates[0]...text` | `.api_key_param()` |

### Semantic Declarations on ServiceProcess

```rust
let svc = ServiceProcess::new("llm")
    .prefix("/api")
    .emits::<LlmResponseEvent>()     // Data Lane
    .faults::<LlmFault>()            // Control Lane
    .manages::<LlmUsageState>()      // Data Lane
    .endpoint(Method::POST, "/chat", post(handler));
```

---

## Advanced Middleware

### Request Timeout

```rust
use vil_server::core::timeout::TimeoutLayer;

VilServer::new("app")
    .layer(TimeoutLayer::from_secs(30))  // 408 after 30s
    .run().await;
```

### API Key Authentication

```rust
use vil_server::auth::api_key::ApiKeyAuth;

let auth = ApiKeyAuth::new().allow_query();
auth.add_key("sk-live-abc123", "Production App");
auth.add_key_scoped("sk-test-xyz", "Test App", vec!["read".to_string()]);

// Validate: X-API-Key header, Authorization: ApiKey <key>, or ?api_key=<key>
```

### IP Allowlist/Blocklist

```rust
use vil_server::auth::ip_filter::IpFilter;

let filter = IpFilter::allowlist()
    .add_cidr("10.0.0.0/8")           // Internal network
    .add_cidr("192.168.1.0/24");       // Office

let block = IpFilter::blocklist()
    .add_ip("1.2.3.4".parse().unwrap()); // Block specific IP
```

### RBAC (Role-Based Access Control)

```rust
use vil_server::auth::rbac::{RbacPolicy, Role};

let policy = RbacPolicy::new();
policy.add_role(Role::new("admin").permission("users:*").permission("orders:*"));
policy.add_role(Role::new("viewer").permission("users:read").permission("orders:read"));

// Check: policy.check_permission(&["admin"], "users:write") → true
// Wildcard: "users:*" matches "users:read", "users:write", etc.
```

### CSRF Protection

```rust
use vil_server::auth::csrf::{CsrfConfig, CsrfProtection};

let csrf = CsrfProtection::new(CsrfConfig::new().exempt_path("/api/webhook"));
let token = csrf.generate_token();
// Double-Submit Cookie: set cookie + require X-CSRF-Token header
// Safe methods (GET, HEAD, OPTIONS) are exempt
```

### Session Management

```rust
use vil_server::auth::session::{SessionManager, SessionConfig};

let mgr = SessionManager::default(); // 30min TTL, HttpOnly, SameSite=Lax
let (session_id, mut data) = mgr.create();
data.set("user_id", serde_json::json!(42));
mgr.update(&session_id, data);

// Cookie: vil-session=<id>; Path=/; Max-Age=1800; HttpOnly; SameSite=Lax
```

### Idempotency (Request Deduplication)

```rust
use vil_server::core::idempotency::IdempotencyStore;

let store = IdempotencyStore::default(); // 24h TTL, 10K max
// Client sends: Idempotency-Key: pay-123
// First: execute handler, cache response
// Duplicate: return cached response (no re-execution)
```

### Middleware Composition Builder

```rust
use vil_server::core::middleware_stack::MiddlewareStack;

let stack = MiddlewareStack::new()
    .timeout(Duration::from_secs(30))
    .compression()
    .security_headers()
    .request_logging()
    .apply(router, &state);
```

---

## Observability

### OpenTelemetry Distributed Tracing

Every request automatically gets a W3C `traceparent` span:

```
traceparent: 00-{trace_id}-{span_id}-01
```

```rust
use vil_server::core::otel::*;

// Spans are auto-collected — no annotation needed
// View: GET /admin/traces → recent spans with trace_id, duration, status
```

### Custom Metrics

```rust
let metrics = state.custom_metrics();
metrics.register_counter("orders_created", "Total orders created");
metrics.register_gauge("active_users", "Current active users");
metrics.register_histogram_default("db_query_ms", "DB query latency");

// Usage in handlers:
metrics.inc("orders_created");
metrics.gauge_set("active_users", 42);
metrics.observe("db_query_ms", 12.5);

// Exported at /metrics alongside auto-generated handler metrics
```

### Runtime Diagnostics

```bash
curl http://localhost:8080/admin/diagnostics
# Returns: server info, runtime state, SHM status, trace stats, error patterns

curl http://localhost:8080/admin/errors
# Returns: error patterns sorted by frequency, recent errors

curl http://localhost:8080/admin/shm
# Returns: SHM region count, capacity, used, utilization %
```

### Alerting Rules

```rust
use vil_server::core::alerting::*;

let mut engine = AlertEngine::new();
engine.add_rule(AlertRule {
    name: "high_error_rate".into(),
    description: "Error rate exceeds 5%".into(),
    severity: AlertSeverity::Critical,
    metric: "error_rate".into(),
    condition: AlertCondition::GreaterThan(5.0),
    for_duration: Duration::from_secs(60),
});
// States: Ok → Pending → Firing → Resolved
```

### API Playground

```bash
# Built-in interactive API explorer:
open http://localhost:8080/admin/playground
# Dark theme, method selector, request body, formatted JSON response
```

---

## WASM & Advanced

### WASM Handler Capabilities

```rust
use vil_server::core::wasm_host::*;

// WASM handlers can access:
// Log, HttpResponse, ReadBody (default)
// ShmRead, ShmWrite, Metrics, MeshSend, KvStore (grant explicitly)

let mut registry = WasmHostRegistry::new();
registry.grant("trusted_handler", WasmCapability::ShmRead);
```

### Pipeline DAG

```rust
use vil_server_mesh::pipeline_dag::*;

let mut dag = PipelineDag::new("etl-pipeline");
dag.add_node(DagNode { id: "ingest".into(), handler: "source".into(), depends_on: vec![], config: None });
dag.add_node(DagNode { id: "validate".into(), handler: "validator".into(), depends_on: vec!["ingest".into()], config: None });
dag.add_node(DagNode { id: "enrich".into(), handler: "enricher".into(), depends_on: vec!["validate".into()], config: None });
dag.add_node(DagNode { id: "log".into(), handler: "logger".into(), depends_on: vec!["validate".into()], config: None });

let plan = dag.plan().unwrap();
// Stage 1: [ingest]
// Stage 2: [validate]
// Stage 3: [enrich, log]  ← parallel execution
```

### Typed RPC (Inter-Service)

```rust
use vil_server_mesh::typed_rpc::RpcRegistry;

let mut rpc = RpcRegistry::new();
rpc.register::<AddRequest, AddResponse, _>("add", |req| {
    AddResponse { result: req.a + req.b }
});

let output = rpc.invoke("add", &serde_json::to_vec(&AddRequest { a: 3, b: 4 }).unwrap()).unwrap();
// Co-located: ~3µs via SHM. Remote: TCP fallback.
```

---

## Production Operations

### In-Memory Cache (LRU + TTL)

```rust
use vil_server::core::cache::Cache;

let cache: Cache<String, serde_json::Value> = Cache::new(10000, Duration::from_secs(300));
cache.put("user:123".into(), json!({"name": "Alice"}));
let user = cache.get(&"user:123".into()); // Some({"name":"Alice"})

let stats = cache.stats();
// { size: 1, hits: 1, misses: 0, hit_rate: 1.0, evictions: 0 }
```

### Feature Flags

```rust
use vil_server::core::feature_flags::FeatureFlags;

let flags = FeatureFlags::new();
flags.define("new_checkout", true, "New checkout flow");
flags.define_rollout("dark_mode", 25, "25% rollout");

if flags.is_enabled("new_checkout") { /* new path */ }
if flags.is_enabled_for("dark_mode", &user_id) { /* gradual rollout */ }
```

### Background Scheduler

```rust
use vil_server::core::scheduler::Scheduler;

let mut sched = Scheduler::new();
sched.every(Duration::from_secs(60), "cleanup", || async {
    cleanup_expired_sessions().await;
});
sched.once(Duration::from_secs(5), "warmup", || async {
    warm_cache().await;
});
```

### API Versioning

```rust
use vil_server::core::api_versioning::ApiVersion;

async fn handler(version: ApiVersion) -> Json<Value> {
    match version.major {
        1 => Json(json!({"api": "v1", "format": "legacy"})),
        2 => Json(json!({"api": "v2", "format": "new"})),
        _ => Json(json!({"error": "unsupported version"})),
    }
}
// Resolved from: URL (/v2/), X-API-Version header, Accept header
```

### Load Balancer & Canary Routing

```rust
use vil_server_mesh::load_balancer::*;

let lb = LoadBalancer::new(
    vec![
        LbEndpoint::new("stable:8080"),
        LbEndpoint::new("canary:8080").canary(),
    ],
    LbStrategy::Canary { canary_weight: 10 }, // 10% to canary
);
let target = lb.next().unwrap(); // automatic selection
```

### Rolling Restart

```rust
use vil_server::core::rolling_restart::RestartCoordinator;

let coord = RestartCoordinator::new(Duration::from_secs(30));
coord.start_drain();       // Stop accepting new requests
coord.wait_for_drain().await; // Wait for in-flight to finish
// → Readiness probe returns "not ready" → LB stops routing
```

---

## Examples

| # | Name | Features |
|---|------|----------|
| 002 | vil-server-hello | Hello world, path params, JSON, SHM info |
| 003 | vil-server-multiservice | 3 services, mesh, visibility, metrics port |
| 004 | vil-server-shm-demo | ShmSlice, blocking_with, SHM stats |
| 005 | vil-server-mesh | Tri-Lane routing, YAML config, ShmDiscovery |
| 006 | vil-server-capsule | WASM dispatch, backpressure, WebSocket |
| 007 | vil-server-wasm | WASM capabilities, DAG planning, features |
| 008 | vil-server-production | Cache, feature flags, API versioning |

---

## DB Semantic Layer (Provider-Neutral, Zero-Cost)

The DB Semantic Layer enables application code to target a provider-neutral database surface. All abstractions are compile-time only — runtime overhead is limited to a single vtable call per query (~1ns).

### Entity Definition

```rust
use vil_db_macros::VilEntity;

#[derive(VilEntity, Serialize, Deserialize)]
#[vil(source = "main_db", table = "orders")]
pub struct Order {
    #[vil(primary_key)]
    pub id: i64,
    pub customer_id: i64,
    pub total: f64,
    pub status: String,
}

// Generates at compile time (zero runtime cost):
// impl VilEntityMeta for Order {
//     const TABLE: &'static str = "orders";
//     const SOURCE: &'static str = "main_db";
//     const PRIMARY_KEY: &'static str = "id";
//     const FIELDS: &'static [&'static str] = &["id", "customer_id", "total", "status"];
// }
```

### Semantic Primitives

```rust
use vil_db_semantic::*;

// All stack-allocated, zero heap:
const MAIN_DB: DatasourceRef = DatasourceRef::new("main_db");
let scope = TxScope::ReadOnly;            // 1 byte
let caps = DbCapability::SQL_STANDARD;     // 4 bytes (u32 bitflag)
let tier = PortabilityTier::P0;            // 1 byte
let cache = CachePolicy::Ttl(60);          // 8 bytes
```

### Datasource Provisioning (Startup-Time)

```rust
let registry = DatasourceRegistry::new();
registry.register("main_db", sqlx_provider, DbCapability::SQL_STANDARD)?;
// Capability mismatch → error at startup, not at runtime

let provider = registry.resolve("main_db")?;  // ~10ns DashMap lookup
let order = provider.find_one("orders", "id", &ToSqlValue::Int(42)).await?;
```

### Provider Switch (Config-Time)

```yaml
# Change provider in vil-server.yaml:
db:
  datasources:
    main_db:
      provider: sea-orm    # was: sqlx
      driver: postgres
      url: "${ENV:DATABASE_URL}"
```

Restart server → provisioning validates capabilities → traffic resumes. No application code changes required for P0 operations.

### Cache Layer (Separated from DB)

```rust
use vil_cache::VilCache;

// SHM backend (co-located, zero-copy):
let cache = ShmCacheBackend::new(shm_query_cache);
cache.set("orders:42", &order_bytes, Some(Duration::from_secs(60))).await;
let cached = cache.get("orders:42").await;

// Redis backend (distributed):
let cache = RedisCacheBackend::new(redis_cache);
cache.set_json("users:42", &user, Some(Duration::from_secs(300))).await;
```

### Portability Tiers

| Tier | Scope | Provider Switch Impact |
|------|-------|----------------------|
| **P0** | CRUD, filter, sort, pagination, transactions | No code changes |
| **P1** | Bulk insert, streaming cursor, aggregates | Capability check required |
| **P2** | Raw SQL, vendor-specific queries | Code review required |

---

## gRPC

```rust
use vil_grpc::GrpcGatewayBuilder;

// 5-line gRPC server:
let gateway = GrpcGatewayBuilder::new()
    .listen(50051)
    .health_check(true)
    .reflection(true);

let server = gateway.build()
    // .add_service(MyServiceServer::new(impl))
    ;
```

Dual HTTP + gRPC in one binary:
```rust
VilServer::new("my-platform")
    .port(8080)       // HTTP
    // .grpc_port(50051) // gRPC (future integration)
    .run().await;
```

---

## Message Queue Adapters

### Kafka

```rust
use vil_mq_kafka::{KafkaProducer, KafkaConsumer, KafkaConfig};

let producer = KafkaProducer::new(KafkaConfig::new("localhost:9092")).await?;
producer.publish("orders.created", order_json.as_bytes()).await?;

let mut consumer = KafkaConsumer::new(
    KafkaConfig::new("localhost:9092").group("my-group").topic("orders.created")
).await?;
consumer.start();
```

### MQTT (IoT)

```rust
use vil_mq_mqtt::{MqttClient, MqttConfig, QoS};

let client = MqttClient::new(
    MqttConfig::new("mqtt://broker:1883").client_id("vil-iot").qos(QoS::AtLeastOnce)
).await?;
client.subscribe("sensors/+/temperature").await?;
client.publish("alerts/high-temp", b"95C", QoS::AtLeastOnce).await?;
```

### NATS (Cloud-Native)

```rust
use vil_mq_nats::{NatsClient, NatsConfig};

let client = NatsClient::connect(NatsConfig::new("nats://localhost:4222")).await?;

// Core pub/sub
client.publish("orders.created", b"order-data").await?;
let mut sub = client.subscribe("orders.>").await?; // wildcard
while let Some(msg) = sub.next().await {
    println!("Got: {} bytes on {}", msg.payload.len(), msg.subject);
}
```

### NATS JetStream (Persistent Streaming)

```rust
use vil_mq_nats::jetstream::{JetStreamClient, StreamConfig, ConsumerConfig};

let js = JetStreamClient::new();
js.create_stream(StreamConfig {
    name: "ORDERS".into(),
    subjects: vec!["orders.>".into()],
    ..Default::default()
}).await?;

let mut consumer = js.create_consumer("ORDERS", ConsumerConfig {
    durable_name: Some("order-processor".into()),
    ..Default::default()
}).await?;

while let Some(msg) = consumer.next().await {
    process(&msg.payload);
    msg.ack().await?; // explicit ack
}
```

### NATS KV Store

```rust
use vil_mq_nats::kv::KvStore;

let kv = KvStore::new("config");
kv.put("feature.dark_mode", b"true").await?;
let entry = kv.get("feature.dark_mode").await.unwrap();

// Watch for real-time changes
let mut watcher = kv.watch();
// watcher.recv() → notified on every put/delete
```

---

## Content Negotiation

```rust
use vil_server_format::FormatResponse;

// Auto-negotiate response format based on Accept header:
async fn list_orders() -> FormatResponse<Vec<Order>> {
    FormatResponse::ok(orders)
}
// Accept: application/json     → JSON
// Accept: application/protobuf → Protobuf (feature-gated)
// Accept: */*                  → JSON (default)
```

---

## CLI Quick Reference

```bash
# Pipeline (lightweight, 5-line gateway)
vil new my-gateway                    # REST gateway
vil new my-gw --type sse             # SSE streaming
vil new my-gw --type grpc            # gRPC proxy
vil init --type rest                  # Init in current dir

# Server (full microservice framework)
vil server new my-api                 # Basic server
vil server new my-api -t nats         # + NATS consumer
vil server new my-api -t kafka        # + Kafka stream
vil server new my-api -t mqtt         # + MQTT IoT gateway
vil server new my-api -t graphql      # + GraphQL + DB
vil server new my-api -t fullstack    # Everything
vil server init -t nats               # Init in current dir

# Operations
vil server dev                        # Dev mode (auto-restart)
vil doctor                            # System readiness check
```

---

*Maintained by VIL Core Team — Vastar Team (vastar.ai) & OceanOS Team (oceanos.id)*
