# VilApp & ServiceProcess

VilApp is the process-oriented application builder; ServiceProcess defines individual service units with typed endpoints and Tri-Lane routing.

## VilApp API

```rust
use vil_server::prelude::*;

VilApp::new("app-name")        // Create application
    .port(8080)                 // HTTP listen port
    .service(service)           // Attach a ServiceProcess
    .mesh(mesh_config)          // Configure Tri-Lane mesh
    .observer(true)             // Enable /_vil/dashboard/
    .run()                      // Start server
    .await;
```

### Methods

| Method | Description |
|--------|-------------|
| `new(name)` | Create app with given name |
| `.port(u16)` | Set HTTP listen port |
| `.profile("prod")` | Apply profile preset (dev/staging/prod) — sets heap_size, tuning |
| `.heap_size(bytes)` | Set ExchangeHeap size in bytes (default: 64MB) |
| `.service(ServiceProcess)` | Add a service process |
| `.mesh(VxMeshConfig)` | Configure inter-service Tri-Lane routes |
| `.plugin(impl VilPlugin)` | Register a plugin |
| `.observer(bool)` | Enable Observer Dashboard |
| `.run().await` | Start the server, block until shutdown |
| `.contract_json()` | Export topology as JSON |

## ServiceProcess API

```rust
let service = ServiceProcess::new("orders")
    .visibility(Visibility::Public)      // Public (HTTP) or Internal (mesh-only)
    .prefix("/api")                      // URL prefix for all endpoints
    .endpoint(Method::GET, "/orders", get(list_orders))
    .endpoint(Method::POST, "/orders", post(create_order))
    .endpoint(Method::GET, "/orders/:id", get(get_order))
    .endpoint(Method::PUT, "/orders/:id", put(update_order))
    .endpoint(Method::DELETE, "/orders/:id", delete(delete_order))
    .extension(db_pool)                  // Inject shared state
    .emits(vec!["OrderCreated"])         // Declares emitted events
    .faults(vec!["OrderNotFound"]);      // Declares fault types
```

### Methods

| Method | Description |
|--------|-------------|
| `new(name)` | Create service with identity |
| `.visibility(Visibility)` | `Public` (HTTP) or `Internal` (mesh-only) |
| `.prefix(path)` | URL prefix for all endpoints |
| `.endpoint(Method, path, handler)` | Register HTTP endpoint |
| `.extension(T)` | Inject shared state (accessible via `ServiceCtx`) |
| `.emits(events)` | Declare events this service produces |
| `.faults(faults)` | Declare faults this service can raise |

## VxMeshConfig (Tri-Lane Routing)

```rust
let mesh = VxMeshConfig::new()
    .route("orders", "payments", VxLane::Data)       // SHM zero-copy
    .route("orders", "auth", VxLane::Trigger)         // Auth check
    .backpressure("payments", 1000);                   // Max in-flight
```

## Multi-Service Example

```rust
#[tokio::main]
async fn main() {
    let auth = ServiceProcess::new("auth")
        .visibility(Visibility::Public)
        .endpoint(Method::POST, "/login", post(login));

    let orders = ServiceProcess::new("orders")
        .prefix("/api")
        .endpoint(Method::GET, "/orders", get(list_orders))
        .extension(store);

    let payments = ServiceProcess::new("payments")
        .visibility(Visibility::Internal);

    let mesh = VxMeshConfig::new()
        .route("orders", "payments", VxLane::Data)
        .route("orders", "auth", VxLane::Trigger);

    VilApp::new("platform")
        .port(8080)
        .service(auth)
        .service(orders)
        .service(payments)
        .mesh(mesh)
        .run()
        .await;
}
```

> Reference: docs/vil/003-VIL-Developer_Guide-Server-Framework.md
