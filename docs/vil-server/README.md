# vil-server — Standalone Compiled Server

vil-server is the **standalone deployment path** for VIL services. Compile multiple services into a single binary with Tri-Lane SHM inter-service communication.

## Architecture

```
┌─────────────────────────────────────────────────┐
│           vil-server (1 compiled binary)       │
│                                                   │
│  ┌──────────┐  ┌──────────┐  ┌──────────────┐   │
│  │ auth     │  │ orders   │  │ payments     │   │
│  │ :Process │  │ :Process │  │ :Process     │   │
│  │ PUBLIC   │  │ PUBLIC   │  │ INTERNAL     │   │
│  └────┬─────┘  └────┬─────┘  └────┬────────┘   │
│       │              │              │             │
│  ═════╪══════════════╪══════════════╪════════════ │
│       │    Tri-Lane SHM (zero-copy)  │            │
│  ═════╪══════════════╪══════════════╪════════════ │
│                                                   │
│  :8080 → HTTP boundary (Axum)                    │
│  :9090 → metrics/health                          │
└─────────────────────────────────────────────────┘
```

## Characteristics

| Aspect | Detail |
|--------|--------|
| **Build** | `cargo build --release` → native binary |
| **Deploy** | Run binary directly |
| **Multi-service** | Yes — compile-time, one binary |
| **Tri-Lane** | Yes — SHM inter-service (zero-copy) |
| **Target** | Development, single-team, simple deployment |
| **Performance** | 2.0M req/s, <1µs mesh latency |

## Usage

```rust
use vil_server::prelude::*;

// Define services as Processes
let auth = ServiceProcess::new("auth")
    .endpoint(Method::GET, "/verify", get(verify_token));

let orders = ServiceProcess::new("orders")
    .prefix("/api")
    .endpoint(Method::POST, "/orders", post(create_order))
    .extension(store);

let payments = ServiceProcess::new("payments")
    .visibility(Visibility::Internal);  // mesh-only

// Configure Tri-Lane mesh
let mesh = VxMeshConfig::new()
    .route("orders", "payments", VxLane::Data)
    .route("orders", "auth", VxLane::Trigger);

// Run
VilApp::new("platform")
    .port(8080)
    .service(auth)
    .service(orders)
    .service(payments)
    .mesh(mesh)
    .run()
    .await;
```

## Crates

| Crate | Purpose |
|-------|---------|
| `vil_server` | Umbrella re-export (single dependency) |
| `vil_server_core` | Core engine: VilApp, ServiceProcess, Tri-Lane mesh |
| `vil_server_web` | Request validation (`Valid<T>`), RFC 7807 errors |
| `vil_server_config` | YAML/ENV config with profiles |
| `vil_server_mesh` | Tri-Lane SHM service mesh |
| `vil_server_auth` | JWT, rate limiting, RBAC, CSRF |
| `vil_server_db` | Database pool trait + transaction wrapper |
| `vil_server_macros` | `#[vil_endpoint]`, `vil_app!`, `#[vil_service]`, `#[vil_service_state]`, `VilWsEvent`, `VilSseEvent` |

## Documentation

- [vil-server Developer Guide](./vil-server-guide.md) — full feature reference
- [Getting Started Tutorial](../tutorials/tutorial-getting-started-server.md) — step-by-step
- [Production Deployment](../tutorials/tutorial-production-server.md) — Docker, Kubernetes
- [API Reference](./API-REFERENCE-SERVER.md) — per-module documentation
- [Examples](../EXAMPLES.md) — 11 server examples (002-016)

> For hot-reload deployment, see vflow-server (licensed separately). Contact the VIL Core Team for details.

## Transpile SDK

Write vil-server endpoints in Python, Go, Java, or TypeScript, then compile to native binary:

```bash
vil compile --from python --input server.py --output my-server --release
```

76 examples in `examples-sdk/` across 4 languages. See [SDK Integration Guide](../vil/SDK-Integration-Guide.md).

## Examples

| # | Example | Focus |
|---|---------|-------|
| 002 | hello-server | Minimal VilApp + ServiceProcess |
| 003 | rest-crud | CRUD + VilModel + HandlerResult |
| 004 | multiservice-mesh | 3 services + VxMeshConfig (Tri-Lane) |
| 005 | shm-zerocopy | ShmSlice + blocking_with |
| 009 | websocket-chat | WebSocket + VilWsEvent + WsHub |
| 010 | graphql-api | GraphQL schema |
| 011 | plugin-database | SqlxConfig + plugins |
| 012 | nats-worker | NATS + JetStream + KV |
| 013 | kafka-stream | Kafka producer/consumer |
| 014 | mqtt-iot-gateway | MQTT IoT bridge |
| 016 | production-fullstack | All features |
