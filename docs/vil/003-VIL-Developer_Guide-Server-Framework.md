# VIL Developer Guide — Part 3: Server Framework

**Series:** VIL Developer Guide (3 of 7)
**Previous:** [Part 2 — Semantic Types & Memory Model](./002-VIL-Developer_Guide-Semantic-Types.md)
**Next:** [Part 4 — Pipeline & HTTP Streaming](./004-VIL-Developer_Guide-Pipeline-Streaming.md)
**Last updated:** 2026-03-26

---

## 1. Process-Oriented Server (Tri-Lane Architecture)

vil-server uses a process-oriented Tri-Lane architecture. Every service is a VIL **Process** communicating via SHM descriptor queues. HTTP is just a boundary — not the architecture.

```
Traditional Microservice (Spring/Quarkus):
  Service A ──HTTP/gRPC──> Service B
  [serialize -> TCP -> deserialize] = ~500us - 2ms per hop

vil-server (co-located):
  Service A ──SHM Tri-Lane──> Service B
  [pointer handoff via /dev/shm] = ~1-5us per hop
```

### 1.1 `VilApp` — Process Topology Builder

Replace `vil_server::new()` with `VilApp::new()` for Process-Oriented server:

```rust
use vil_server::prelude::*;

#[tokio::main]
async fn main() {
    let service = ServiceProcess::new("hello")
        .endpoint(Method::GET, "/", get(hello))
        .endpoint(Method::GET, "/greet/:name", get(greet));

    VilApp::new("hello-server")
        .port(8080)
        .service(service)
        .run()
        .await;
}
```

**See:** [`002-basic-usage-hello-server`](../../examples/002-basic-usage-hello-server/src/main.rs)

### 1.2 `ServiceProcess` — Service as Process

Each service is a VIL Process with typed ports and Tri-Lane routing:

```rust
// ── Step 1: Define services (can breathe here) ──
let auth = ServiceProcess::new("auth")
    .visibility(Visibility::Public)
    .endpoint(Method::POST, "/login", post(login))
    .endpoint(Method::GET, "/verify", get(verify));

let orders = ServiceProcess::new("orders")
    .prefix("/api")
    .endpoint(Method::GET, "/orders", get(list_orders))
    .endpoint(Method::POST, "/orders", post(create_order))
    .extension(store);  // inject shared state

let payments = ServiceProcess::new("payments")
    .visibility(Visibility::Internal);  // mesh-only, no HTTP
```

**Dot-builder can be broken** — each service is its own variable.

**See:** [`003-basic-usage-rest-crud`](../../examples/003-basic-usage-rest-crud/src/main.rs) — CRUD with ServiceProcess + Extension.

### 1.3 `VxMeshConfig` — Tri-Lane Inter-Service Routing

Declare SHM Tri-Lane routes between services:

```rust
// ── Step 2: Configure mesh (separate concern) ──
let mesh = VxMeshConfig::new()
    .route("orders", "payments", VxLane::Data)      // SHM zero-copy
    .route("orders", "auth", VxLane::Trigger)        // auth check
    .backpressure("payments", 1000);                  // max in-flight

// ── Step 3: Assemble ──
VilApp::new("platform")
    .port(8080)
    .service(auth)
    .service(orders)
    .service(payments)
    .mesh(mesh)
    .run()
    .await;
```

**See:** [`004-basic-usage-multiservice-mesh`](../../examples/004-basic-usage-multiservice-mesh/src/main.rs) — 3 services with Tri-Lane mesh + Core Banking SSE streaming.

### 1.4 Cross-Host Transport

VIL Tri-Lane communication supports two transport modes, selected transparently:

- **SHM Tri-Lane** — for co-located services on the same host (~50ns zero-copy)
- **TCP Tri-Lane** — for remote services across hosts (~50-500µs)
- **`Transport::auto()`** — transparent selection: SHM when co-located, TCP when remote

The TCP wire protocol uses **length-prefixed binary framing** for minimal overhead. `TcpTriLaneRouter` manages peer connections with persistent sockets and automatic reconnect.

```rust
// Cross-host mesh: router listens and connects to remote peers
let router = TcpTriLaneRouter::new("0.0.0.0:9090")
    .add_peer("svc-b", "10.0.1.5:9090");
```

### 1.5 `#[vil_endpoint]` — Endpoint Process Annotation

Mark handler functions as endpoint Processes:

```rust
#[vil_endpoint]                        // default: AsyncTask
async fn get_order(Path(id): Path<u64>) -> VilResult<Order> { ... }

#[vil_endpoint(exec = BlockingTask)]   // CPU-bound: spawn_blocking
fn score_model(body: ScoreInput) -> VilResult<ScoreOutput> { ... }

#[vil_endpoint(exec = DedicatedThread)] // isolated worker
fn heavy_compute(body: Data) -> VilResult<Result> { ... }
```

ExecClass options: `AsyncTask` (default), `BlockingTask`, `DedicatedThread`, `PinnedWorker`, `WasmCapsule`.

### 1.6 `vil_app!` — Declarative DSL

The simplest way to define a VIL app — no builders needed:

```rust
use vil_server::prelude::*;

async fn hello() -> &'static str { "Hello!" }
async fn create_order(Json(body): Json<Order>) -> VilResponse<Order> {
    VilResponse::created(body)
}

vil_app! {
    name: "order-service",
    port: 8080,
    endpoints: {
        GET  "/"              => hello,
        POST "/api/orders"    => create_order,
        GET  "/api/orders/:id" => get_order,
    }
}
```

Generates `main()` with ServiceProcess + VilApp automatically.

### 1.7 Contract Export

Every VilApp can export its Process topology as JSON:

```rust
let app = VilApp::new("platform")
    .service(auth)
    .service(orders)
    .mesh(mesh);

println!("{}", app.contract_json());
// {"name":"platform","architecture":"VX Process-Oriented (Tri-Lane)",...}
```

---

## 2. VX Architecture Deep Dive

### 2.1 Request Flow

```
                    HTTP Request
                         |
                         v
+--------------------------------------------------------+
|  HttpIngress (IngressBridge)                           |
|    Axum accept -> parse -> RequestDescriptor (40B)     |
|    -> write to ExchangeHeap -> push to Trigger Lane    |
+----+---------------------------------------------------+
     |
     v  Trigger Lane (SHM)
+--------------------------------------------------------+
|  VxKernel (Token-Based Executor)                       |
|    Phase 1: exhaust  (drain pending tokens)            |
|    Phase 2: check    (timeout, health)                 |
|    Phase 3: park     (wait for new work)               |
+----+---------------------------------------------------+
     |
     v  Data Lane (SHM)
+--------------------------------------------------------+
|  ServiceProcess (handler execution)                    |
|    ExecClass determines execution strategy:            |
|      AsyncTask       -> tokio::spawn                   |
|      BlockingTask    -> spawn_blocking                  |
|      DedicatedThread -> std::thread::spawn              |
|      PinnedWorker    -> tokio::spawn on pinned core    |
|      WasmCapsule     -> WASM runtime                   |
+----+---------------------------------------------------+
     |
     v  Data Lane (SHM) -> ResponseDescriptor (24B)
+--------------------------------------------------------+
|  HttpEgress (EgressHandle)                             |
|    Data Lane -> read ResponseDescriptor from SHM       |
|    -> format HTTP response -> send to client           |
+--------------------------------------------------------+
```

### 2.2 VASI-Compliant Descriptors

| Descriptor | Size | Fields |
|-----------|------|--------|
| `RequestDescriptor` | 40 bytes | session_id, method, path_hash, content_type, body_offset, body_len, timestamp_ns |
| `ResponseDescriptor` | 24 bytes | session_id, status_code, body_offset, body_len |

Both are `#[repr(C)]` for cross-language compatibility.

### 2.3 VX vs V5 Comparison

| Aspect | V5 (VilServer) | VX (VilApp) |
|--------|-------------------|----------------|
| Entry point | `VilServer::new()` | `VilApp::new()` |
| Service unit | Route handler (function) | `ServiceProcess` (identity + ports + failure domain) |
| Mesh config | `.mesh(\|m\| m.route(...))` | `VxMeshConfig::new().route(...)` |
| Executor | Tokio only | `ExecClass` (5 modes) |
| Request tracking | RequestId (String) | `RequestDescriptor` (40B, VASI, #[repr(C)]) |
| Kernel | None (Axum internal) | `VxKernel` (token-based, 3-phase) |
| Cleanup | Graceful shutdown only | `CleanupConfig` (orphan TTL, auto-restart) |

---

## 3. Database Integration

### 3.1 Plugin-Based Architecture (V6)

All database drivers are pre-compiled into the binary but disabled by default. Activation, configuration, and monitoring are done via Admin GUI without restart.

| Plugin Crate | Technology | Features |
|-------------|-----------|----------|
| `vil_db_sqlx` | PostgreSQL/MySQL/SQLite | Compile-time checked queries, connection pooling, SHM result cache |
| `vil_db_sea_orm` | Full ORM | Migration runner, query builder, lazy/eager loading, soft delete |
| `vil_db_redis` | Redis KV + Pub/Sub | Connection pooling, cache helpers, pub/sub bridge to EventBus |

### 3.2 DB Semantic Layer (V7) — Zero-Cost Abstraction

Semantic DB layer adds **zero overhead** per query (~11ns = 1 vtable call):

```rust
#[derive(VilEntity, Serialize, Deserialize)]
#[vil(source = "main_db", table = "orders")]
pub struct Order {
    #[vil(primary_key)]
    pub id: i64,
    pub customer_id: i64,
    pub total: f64,
    pub status: String,
    #[vil(created_at)]
    pub created_at: i64,
}

// Auto-generated (const, zero runtime cost):
impl VilEntityMeta for Order {
    const TABLE: &'static str = "orders";
    const SOURCE: &'static str = "main_db";
    const PRIMARY_KEY: &'static str = "id";
    const FIELDS: &'static [&'static str] = &["id", "customer_id", "total", "status", "created_at"];
    const PORTABILITY: PortabilityTier = PortabilityTier::P0;
}
```

### 3.3 Repository Pattern

```rust
#[vil_repository(source = "main_db")]
pub trait OrderRepo {
    async fn find_by_id(&self, id: i64) -> DbResult<Option<Order>>;
    async fn list_by_customer(&self, customer_id: i64) -> DbResult<Vec<Order>>;
    async fn save(&self, order: &Order) -> DbResult<()>;
    async fn delete(&self, id: i64) -> DbResult<bool>;
}

// Handler usage (zero-cost):
async fn list_orders(
    repo: Repo<dyn OrderRepo>,
) -> Json<Vec<Order>> {
    let orders = repo.list_by_customer(42).await?;
    Json(orders)
}
```

### 3.4 Portability Tiers

| Tier | Operations | Provider Switch |
|------|-----------|-----------------|
| **P0** (Portable Core) | CRUD, filter, sort, pagination, tx | No code changes |
| **P1** (Capability-Gated) | BulkInsert, StreamingCursor, Aggregate | Needs capability check |
| **P2** (Provider-Specific) | JSON path query, full text search, raw SQL | Needs code review |

### 3.5 Config Encryption

Database URLs are encrypted at rest via `SecretResolver` (auto-detect by prefix):

| Prefix | Provider |
|--------|----------|
| `ENC[AES256:...]` | Local AES-256-GCM (key at `~/.vil/secrets/`) |
| `${ENV:VAR_NAME}` | Environment variable |
| `${K8S_SECRET:name/key}` | Kubernetes Secrets API |
| `${VAULT:path#key}` | HashiCorp Vault HTTP API |

---

## 4. Multi-Service Deployment Model

### 4.1 Process Monolith (Default)

```
+-----------------------------------------------------+
|            vil-server (1 binary)                   |
|                                                     |
|  +----------+  +----------+  +------------------+  |
|  | auth     |  | orders   |  | payments         |  |
|  | :process |  | :process |  | :process         |  |
|  | PUBLIC   |  | PUBLIC   |  | INTERNAL (mesh)  |  |
|  +----+-----+  +----+-----+  +----+-------------+  |
|       |              |              |                |
|  =====+==============+==============+===========    |
|       |    /dev/shm/vil_mesh_*    |                |
|       |    (zero-copy IPC via SHM)  |                |
|  =====+==============+==============+===========    |
|                                                     |
|  :8080 -> public API (auth, orders)                 |
|  :9090 -> metrics/health (internal)                 |
+-----------------------------------------------------+
```

### 4.2 Tri-Lane in Microservice Context

```
+---------------------------------------------------------+
|                    Tri-Lane Mesh                        |
+---------------+------------------+----------------------+
| TRIGGER LANE  |    DATA LANE     |   CONTROL LANE       |
+---------------+------------------+----------------------+
| Request init  | Payload stream   | Backpressure signal  |
| Auth token    | Response body    | Circuit breaker      |
| Session start | SSE chunks       | Rate limit feedback  |
| Route decision| File upload      | Graceful drain       |
|               | (zero-copy SHM)  | Health status        |
+---------------+------------------+----------------------+

Key advantage: Control Lane is NEVER blocked by Data Lane congestion.
```

### 4.3 YAML Service Definition

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

profiles:
  dev:
    log_level: debug
    workers: 1
  prod:
    log_level: warn
    workers: 8
```

---

## 5. Layer API Reference

| Layer | API | Use Case | Example |
|-------|-----|----------|---------|
| Layer 1 | `vil_app!` macro | Declarative DSL, minimal code | (embedded main) |
| Layer 2 | `ServiceProcess` + `VilApp` builder | Multi-service, custom topology | 002, 003, 004, 005, 009-014, 016 |
| Layer 3 | `vil_workflow!` (pipeline) | Full control, multi-node pipeline | 001, 006-008, 015, 017, 018 |

---

## 6. Configuration System

Precedence (highest to lowest):

```
1. Command-line flags     (--port 8080)
2. Environment variables  (VIL_SERVER_PORT=8080)
3. Profile-specific file  (vil-server.prod.yaml)
4. Default config file    (vil-server.yaml)
5. Built-in defaults      (port=3080, workers=num_cpus)
```

Key environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `VIL_SERVER_PORT` | 3080 | HTTP listen port |
| `VIL_SERVER_HOST` | 0.0.0.0 | Bind address |
| `VIL_METRICS_PORT` | 9090 | Prometheus metrics port |
| `VIL_LOG_LEVEL` | info | Log level |
| `VIL_PROFILE` | dev | Active profile |
| `VIL_WORKERS` | num_cpus | Worker thread count |
| `VIL_REQUEST_TIMEOUT` | 30 | Request timeout in seconds |
| `VIL_JWT_SECRET` | -- | JWT signing secret |

---

## What's New (2026-03-26)

### The VIL Way: Default Extractors & Response Types

The server framework now enforces "The VIL Way" pattern as the default for all handlers:

| Old Pattern | VIL Way Replacement | Why |
|-------------|-------------------|-----|
| `Json<T>` body extractor | `ShmSlice` | Zero-copy by default; JSON bodies are written to SHM on ingress |
| `Extension<Arc<T>>` state | `ServiceCtx` + `ctx.state::<T>()` | Semantic context with Tri-Lane metadata |
| `serde_json::to_vec()` | `vil_json::to_bytes()` | SIMD-accelerated (sonic-rs) when `simd` feature enabled |
| `Json(response)` | `VilResponse<T>` / `ShmVilResponse<T>` | Typed envelope with SHM write-through option |

```rust
// The VIL Way handler:
#[vil_handler(shm)]
async fn create_order(ctx: ServiceCtx, slice: ShmSlice) -> VilResponse<Order> {
    let input: CreateOrder = vil_json::from_slice(slice.as_ref())?;
    let db = ctx.state::<DbPool>();
    let order = db.insert(&input).await?;
    VilResponse::created(order)
}
```

### `ServiceName` Newtype

`VilApp::run()` now injects a `ServiceName` newtype into every `ServiceCtx`. This enables per-service tracing spans, metrics labels, and log prefixes without manual configuration:

```rust
/// Newtype for compile-time service identity.
pub struct ServiceName(pub &'static str);

// Injected automatically by VilApp::run():
let name = ctx.service_name(); // → &ServiceName
tracing::info!(service = %name, "handling request");
```

### `ShmVilResponse<T>` for SHM Write-Through

For handlers that produce large responses, `ShmVilResponse<T>` writes the response body directly to SHM before sending the HTTP response. Downstream services on the same host can read via SHM zero-copy instead of re-parsing:

```rust
#[vil_handler(shm)]
async fn heavy_query(ctx: ServiceCtx) -> ShmVilResponse<LargeReport> {
    let report = generate_report().await;
    // Body is written to ExchangeHeap; HTTP response carries SHM offset
    ShmVilResponse::ok(report)
}
```

### axum 0.7 Unification

All server crates now use **axum 0.7** exclusively. The legacy axum 0.6 dependency has been removed, eliminating the dual-version `axum-core` conflict that previously caused compile warnings. Key changes:
- `axum::extract::Extension` is deprecated in favor of `axum::extract::State` (wrapped by `ServiceCtx`)
- `IntoResponse` trait updated to 0.7 signature
- `Router::merge()` behavior is now consistent across all server crates

---

### Phase 6: All Handlers Use ServiceCtx + ShmSlice

As of 2026-03-26, all **51 AI plugin crates** and all server examples use the VIL Way handler pattern exclusively:

- **`ServiceCtx`** for typed state access (replaces `Extension<Arc<T>>`) — 51/51 crates
- **`ShmSlice`** for zero-copy body extraction (replaces `Json<T>`) — 51/51 crates
- **`ctx.state::<T>()`** for shared state (replaces `.extension()`) — 51/51 crates
- **Zero `Extension<T>` remaining** across the entire codebase

This means every handler in the VIL ecosystem follows this pattern:

```rust
#[vil_handler(shm)]
async fn process(ctx: ServiceCtx, slice: ShmSlice) -> VilResponse<Output> {
    let state = ctx.state::<AppState>();
    // ...
}
```

---

*Previous: [Part 2 — Semantic Types & Memory Model](./002-VIL-Developer_Guide-Semantic-Types.md)*
*Next: [Part 4 — Pipeline & HTTP Streaming](./004-VIL-Developer_Guide-Pipeline-Streaming.md)*
