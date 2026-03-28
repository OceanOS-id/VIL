# VX_APP Pattern

VX_APP is the process-oriented server pattern using ShmSlice body extraction, ServiceCtx state access, and VilResponse output.

## Core Handler Pattern

```rust
use vil_server::prelude::*;

#[vil_handler(shm)]
async fn create_order(ctx: ServiceCtx, slice: ShmSlice) -> VilResponse<Order> {
    let input: CreateOrder = vil_json::from_slice(slice.as_ref())?;
    let db = ctx.state::<DbPool>();
    let order = db.insert(&input).await?;
    VilResponse::created(order)
}
```

## ShmSlice (Body Extraction)

Replaces `Json<T>` with zero-copy SHM body extraction:

```rust
// ShmSlice is auto-extracted from request body by #[vil_handler(shm)]
async fn handler(slice: ShmSlice) -> VilResponse<Output> {
    let data: MyType = slice.json()?;        // Deserialize JSON
    let text = slice.text()?;                 // As UTF-8 string
    let raw = slice.as_bytes();               // Raw bytes
    VilResponse::ok(process(data))
}
```

## ServiceCtx (State Access)

Replaces `Extension<Arc<T>>` with typed Tri-Lane context:

```rust
async fn handler(ctx: ServiceCtx) -> VilResponse<Output> {
    let db = ctx.state::<DbPool>();           // Typed state access
    let name = ctx.service_name();            // Service identity
    let sid = ctx.session_id();               // Tri-Lane session ID
    // ...
}
```

## VilResponse (Output)

SIMD-accelerated JSON response via `vil_json`:

```rust
VilResponse::ok(data)           // 200 OK
VilResponse::created(data)      // 201 Created
VilResponse::with_shm(data)     // Write-through to SHM
```

## ServiceProcess

Each service is a named VIL Process:

```rust
let api = ServiceProcess::new("api")
    .visibility(Visibility::Public)
    .endpoint(Method::GET, "/orders", get(list_orders))
    .endpoint(Method::POST, "/orders", post(create_order))
    .endpoint(Method::GET, "/orders/:id", get(get_order))
    .extension(db_pool);  // Inject shared state
```

## VilApp Assembly

```rust
#[tokio::main]
async fn main() {
    let api = ServiceProcess::new("api")
        .endpoint(Method::GET, "/", get(hello));

    let internal = ServiceProcess::new("worker")
        .visibility(Visibility::Internal);

    let mesh = VxMeshConfig::new()
        .route("api", "worker", VxLane::Data);

    VilApp::new("my-app")
        .port(8080)
        .observer(true)     // /_vil/dashboard/ + /_vil/api/*
        .service(api)
        .service(internal)
        .mesh(mesh)
        .run()
        .await;
}
```

## Observer Dashboard

Enable with `.observer(true)`. Serves:
- `/_vil/dashboard/` — browser-accessible dark-theme SPA
- `/_vil/api/topology` — service topology + endpoint metrics
- `/_vil/api/system` — OS-level metrics (pid, cpu, memory, threads)
- `/_vil/api/routes` — registered routes with exec_class
- `/_vil/api/shm` — SHM pool stats
- `/_vil/api/config` — running config

```rust
VilApp::new("app")
    .port(8080)
    .observer(true)
    .service(my_service)
    .run()
    .await;
```

## Error Handling

```rust
async fn get_order(Path(id): Path<u64>) -> HandlerResult<VilResponse<Order>> {
    let order = find_order(id)
        .ok_or_else(|| VilError::not_found(format!("Order {} not found", id)))?;
    Ok(VilResponse::ok(order))
}
```

> Reference: docs/vil/003-VIL-Developer_Guide-Server-Framework.md
