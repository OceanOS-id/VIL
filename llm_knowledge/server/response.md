# VilResponse & VilError

VilResponse is the typed JSON response envelope; VilError provides RFC 7807 structured errors.

## VilResponse<T>

```rust
use vil_server::prelude::*;

#[derive(Serialize)]
struct Order { id: u64, total: f64 }

// 200 OK
async fn get_order() -> VilResponse<Order> {
    VilResponse::ok(Order { id: 1, total: 99.50 })
}

// 201 Created
async fn create_order() -> VilResponse<Order> {
    VilResponse::created(Order { id: 2, total: 150.00 })
}
```

### Methods

| Method | Status | Description |
|--------|--------|-------------|
| `VilResponse::ok(data)` | 200 | Standard success |
| `VilResponse::created(data)` | 201 | Resource created |
| `VilResponse::with_shm(data)` | 200 | Write-through to SHM |

## ShmVilResponse<T>

For large responses that downstream services read via SHM zero-copy:

```rust
#[vil_handler(shm)]
async fn heavy_query(ctx: ServiceCtx) -> ShmVilResponse<LargeReport> {
    let report = generate_report().await;
    ShmVilResponse::ok(report)
    // Body written to ExchangeHeap; HTTP response carries SHM offset
}
```

## VilError

RFC 7807 Problem Detail errors with factory methods:

```rust
async fn get_order(Path(id): Path<u64>) -> HandlerResult<VilResponse<Order>> {
    let order = find_order(id)
        .ok_or_else(|| VilError::not_found(format!("Order {} not found", id)))?;
    Ok(VilResponse::ok(order))
}
```

### Factory Methods

| Method | Status | Usage |
|--------|--------|-------|
| `VilError::bad_request(msg)` | 400 | Invalid input |
| `VilError::not_found(msg)` | 404 | Resource missing |
| `VilError::unauthorized(msg)` | 401 | Auth required |
| `VilError::forbidden(msg)` | 403 | Insufficient permissions |
| `VilError::internal(msg)` | 500 | Server error |
| `VilError::validation(msg)` | 422 | Validation failure |
| `VilError::rate_limited(msg)` | 429 | Rate limit exceeded |
| `VilError::service_unavailable(msg)` | 503 | Service down |

## HandlerResult<T>

Type alias for `Result<T, VilError>`:

```rust
async fn handler() -> HandlerResult<VilResponse<Order>> {
    let order = db.find(id).await
        .map_err(|e| VilError::internal(e.to_string()))?;
    Ok(VilResponse::ok(order))
}
```

## Custom Error Enums

```rust
#[derive(Debug, DeriveVilError)]
enum OrderError {
    #[vil_error(status = 404)]
    NotFound { id: u64 },
    #[vil_error(status = 400)]
    InvalidTotal,
    #[vil_error(status = 500)]
    DbError(String),
}

// Auto-generates From<OrderError> for VilError
async fn handler() -> HandlerResult<VilResponse<Order>> {
    Err(OrderError::NotFound { id: 42 }.into())
}
```

> Reference: docs/vil/002-VIL-Developer_Guide-Semantic-Types.md
