# VIL Extractors

All VIL extractors for request handling: ShmSlice, ServiceCtx, RequestId, and ShmContext.

## ShmSlice (Zero-Copy Body)

Extracts the HTTP request body via SHM zero-copy. Replaces `Json<T>`.

```rust
use vil_server::prelude::*;

#[vil_handler(shm)]
async fn handler(slice: ShmSlice) -> VilResponse<Output> {
    // Deserialize JSON from SHM bytes
    let data: MyType = slice.json()?;

    // Or as UTF-8 string
    let text: &str = slice.text()?;

    // Or raw bytes
    let raw: &[u8] = slice.as_bytes();

    VilResponse::ok(process(data))
}
```

### ShmSlice Methods

| Method | Return | Description |
|--------|--------|-------------|
| `.json::<T>()` | `Result<T>` | Deserialize JSON (via `vil_json`) |
| `.text()` | `Result<&str>` | UTF-8 string view |
| `.as_bytes()` | `&[u8]` | Raw byte slice |
| `.len()` | `usize` | Body length |
| `.is_empty()` | `bool` | Check if body is empty |

## ServiceCtx (State Access)

Typed state access with Tri-Lane metadata. Replaces `Extension<Arc<T>>`.

```rust
#[vil_handler(shm)]
async fn handler(ctx: ServiceCtx, slice: ShmSlice) -> VilResponse<Output> {
    // Access typed shared state
    let db = ctx.state::<DbPool>();

    // Service identity (auto-injected by VilApp)
    let name: &ServiceName = ctx.service_name();

    // Tri-Lane session ID
    let session: u64 = ctx.session_id();

    // Current lane kind
    let lane: LaneKind = ctx.lane();

    // Send to another service via mesh
    ctx.send("payments", payload).await?;

    // Emit trigger signal
    ctx.trigger("auth", token).await?;

    // Send control signal
    ctx.control(ControlSignal::Done).await?;

    VilResponse::ok(output)
}
```

### ServiceCtx Methods

| Method | Return | Description |
|--------|--------|-------------|
| `.state::<T>()` | `&T` | Access typed state |
| `.service_name()` | `&ServiceName` | Service identity |
| `.session_id()` | `u64` | Current session ID |
| `.lane()` | `LaneKind` | Current Tri-Lane |
| `.send(target, data)` | `Result<()>` | Send via Data Lane |
| `.trigger(target, data)` | `Result<()>` | Send via Trigger Lane |
| `.control(signal)` | `Result<()>` | Send via Control Lane |

## RequestId

Auto-extracted from `X-Request-Id` header:

```rust
async fn handler(request_id: RequestId) -> VilResponse<Output> {
    tracing::info!(request_id = %request_id, "processing");
    // ...
}
```

## ShmContext

Access ExchangeHeap stats:

```rust
async fn handler(shm: ShmContext) -> VilResponse<ShmStats> {
    let stats = shm.stats();
    VilResponse::ok(ShmStats {
        used_bytes: stats.used,
        free_bytes: stats.free,
        regions: stats.region_count,
    })
}
```

## Standard Axum Extractors

VIL is built on axum 0.7 -- standard extractors work alongside VIL extractors:

```rust
async fn handler(
    Path(id): Path<u64>,
    Query(params): Query<ListParams>,
    headers: HeaderMap,
) -> VilResponse<Output> {
    // ...
}
```

> Reference: docs/vil/002-VIL-Developer_Guide-Semantic-Types.md
