# Server Macros

All VIL server macros for handler annotation, endpoint configuration, and app declaration.

## #[vil_handler]

Wraps handlers with RequestId injection, tracing spans, and error mapping:

```rust
use vil_server::prelude::*;

#[vil_handler]
async fn get_user(id: Path<u64>) -> Result<User, AppError> {
    let user = db::find_user(*id).await?;
    Ok(user)
}
// Generated:
//   - RequestId auto-injected
//   - tracing::info_span!("get_user", request_id = ...)
//   - Ok(data) -> VilResponse::ok(data)
//   - Err(e) -> VilError via Into<VilError>
```

## #[vil_handler(shm)]

Adds ShmSlice body extraction and ServiceCtx injection:

```rust
#[vil_handler(shm)]
async fn process(ctx: ServiceCtx, slice: ShmSlice) -> VilResponse<Output> {
    let state = ctx.state::<AppState>();
    let input: Input = vil_json::from_slice(slice.as_ref())?;
    VilResponse::ok(compute(state, input))
}
// Generated:
//   - ShmSlice extracted from request body
//   - ServiceCtx with Tri-Lane metadata
//   - Tracing span with request_id + service_name
```

## #[vil_endpoint]

Marks handlers with execution class:

```rust
#[vil_endpoint]                          // default: AsyncTask
async fn query(Path(id): Path<u64>) -> VilResult<Order> { ... }

#[vil_endpoint(exec = BlockingTask)]     // CPU-bound -> spawn_blocking
fn score(body: ScoreInput) -> VilResult<ScoreOutput> { ... }

#[vil_endpoint(exec = DedicatedThread)]  // Isolated worker thread
fn compute(body: Data) -> VilResult<Result> { ... }
```

### ExecClass Options

| Class | Runtime | Use Case |
|-------|---------|----------|
| `AsyncTask` | `tokio::spawn` | Default, I/O-bound |
| `BlockingTask` | `spawn_blocking` | CPU-bound, short |
| `DedicatedThread` | `std::thread::spawn` | Long-running |
| `PinnedWorker` | Pinned to CPU core | Latency-critical |
| `WasmCapsule` | WASM runtime | Sandboxed execution |

## vil_app! DSL

Declarative app definition -- generates main() with ServiceProcess + VilApp:

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

## #[vil_service]

Annotate a struct as a service with lifecycle hooks:

```rust
#[vil_service]
struct OrderService {
    db: DbPool,
}

impl OrderService {
    async fn on_start(&self) { /* initialization */ }
    async fn on_stop(&self) { /* cleanup */ }
}
```

## #[derive(VilModel)]

Zero-copy data model with SHM serialization:

```rust
#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct Task {
    id: u64,
    title: String,
    done: bool,
}

let bytes = task.to_json_bytes()?;              // Serialize to Bytes
let task: Task = Task::from_shm_bytes(&bytes)?; // Deserialize from SHM
```

## #[derive(VilSseEvent)]

SSE event helpers for broadcasting:

```rust
#[derive(Serialize, VilSseEvent)]
#[sse_event(topic = "order_update")]
struct OrderUpdated { order_id: u64, status: String }

let event = order.to_sse_event()?;   // -> axum sse::Event
order.broadcast(&sse_hub);           // Broadcast to subscribers
```

> Reference: docs/vil/002-VIL-Developer_Guide-Semantic-Types.md
