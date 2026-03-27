# Recipe: Multi-Service Fan-Out

Multiple services with shared ExchangeHeap communicating via Tri-Lane mesh.

## Full Example

```rust
use vil_server::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct Order { id: u64, amount: f64, status: String }

#[tokio::main]
async fn main() {
    // Service 1: Public API
    let api = ServiceProcess::new("api")
        .visibility(Visibility::Public)
        .prefix("/api")
        .endpoint(Method::POST, "/orders", post(create_order))
        .endpoint(Method::GET, "/orders/:id", get(get_order));

    // Service 2: Payment processing (internal only)
    let payments = ServiceProcess::new("payments")
        .visibility(Visibility::Internal)
        .extension(PaymentGateway::new());

    // Service 3: Notification (internal only)
    let notify = ServiceProcess::new("notify")
        .visibility(Visibility::Internal)
        .extension(EmailClient::new());

    // Tri-Lane mesh: API fans out to payments + notify
    let mesh = VxMeshConfig::new()
        .route("api", "payments", VxLane::Data)
        .route("api", "notify", VxLane::Trigger)
        .backpressure("payments", 500);

    VilApp::new("order-platform")
        .port(8080)
        .service(api)
        .service(payments)
        .service(notify)
        .mesh(mesh)
        .run()
        .await;
}
```

## Create Order (Fan-Out)

```rust
#[vil_handler(shm)]
async fn create_order(ctx: ServiceCtx, slice: ShmSlice) -> VilResponse<Order> {
    let input: CreateOrder = slice.json()?;
    let order = Order {
        id: generate_id(),
        amount: input.amount,
        status: "pending".to_string(),
    };

    // Fan-out: send to payments via Data Lane (zero-copy)
    ctx.send("payments", &order).await?;

    // Fan-out: trigger notification via Trigger Lane
    ctx.trigger("notify", &order).await?;

    VilResponse::created(order)
}
```

## Get Order

```rust
async fn get_order(ctx: ServiceCtx, Path(id): Path<u64>) -> HandlerResult<VilResponse<Order>> {
    let store = ctx.state::<OrderStore>();
    let order = store.get(id)
        .ok_or_else(|| VilError::not_found(format!("Order {} not found", id)))?;
    Ok(VilResponse::ok(order))
}
```

## Pipeline Fan-Out Alternative

Same pattern using `vil_workflow!` for streaming:

```rust
let source = HttpSourceBuilder::new()
    .url("http://upstream/orders/stream")
    .format(HttpFormat::SSE)
    .build();

let sink_payments = HttpSinkBuilder::new().port(3081).path("/pay").build();
let sink_notify = HttpSinkBuilder::new().port(3082).path("/notify").build();

let (_ir, handles) = vil_workflow! {
    name: "OrderFanOut",
    token: ShmToken,
    instances: [ source, sink_payments, sink_notify ],
    routes: [
        source.data -> sink_payments.in (LoanWrite),
        source.data -> sink_notify.in (Copy),
    ]
};
```

## Tri-Lane Reference

| Lane | Purpose | Transport |
|------|---------|-----------|
| Data | Business payloads | SHM zero-copy (~1-5us) |
| Trigger | Signals, commands | Lightweight notification |
| Control | Backpressure, shutdown | System coordination |

## Test

```bash
# Create order (fans out to payments + notify)
curl -X POST http://localhost:8080/api/orders \
  -d '{"amount": 99.50}'

# Response
{"id":1,"amount":99.50,"status":"pending"}
```

> Reference: docs/vil/003-VIL-Developer_Guide-Server-Framework.md
