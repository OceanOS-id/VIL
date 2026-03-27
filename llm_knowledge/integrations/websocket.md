# WebSocket & SSE Server

WsHub for WebSocket broadcast, SseHub for SSE push, both with topic-based routing.

## WebSocket (WsHub)

```rust
use vil_server::prelude::*;
use vil_server::websocket::{WsHub, WsMessage};

let ws_hub = WsHub::new();

let service = ServiceProcess::new("ws")
    .extension(ws_hub.clone())
    .endpoint(Method::GET, "/ws", get(ws_handler));

VilApp::new("ws-server")
    .port(8080)
    .service(service)
    .run()
    .await;
```

### WebSocket Handler

```rust
async fn ws_handler(ws: WebSocketUpgrade, ctx: ServiceCtx) -> impl IntoResponse {
    let hub = ctx.state::<WsHub>().clone();
    ws.on_upgrade(|socket| async move {
        hub.add_client(socket, "chat").await;
    })
}
```

### Broadcast

```rust
#[vil_handler(shm)]
async fn send_message(ctx: ServiceCtx, slice: ShmSlice) -> VilResponse<()> {
    let msg: ChatMessage = slice.json()?;
    let hub = ctx.state::<WsHub>();
    hub.broadcast("chat", WsMessage::text(&msg.content)).await;
    VilResponse::ok(())
}
```

## SSE Server (SseHub)

```rust
use vil_server::sse::{SseHub, SseEvent};

let sse_hub = SseHub::new();

let service = ServiceProcess::new("sse")
    .extension(sse_hub.clone())
    .endpoint(Method::GET, "/events", get(sse_stream))
    .endpoint(Method::POST, "/emit", post(emit_event));
```

### SSE Stream Endpoint

```rust
async fn sse_stream(ctx: ServiceCtx) -> impl IntoResponse {
    let hub = ctx.state::<SseHub>();
    hub.subscribe("updates")
}
```

### Emit SSE Event

```rust
#[vil_handler(shm)]
async fn emit_event(ctx: ServiceCtx, slice: ShmSlice) -> VilResponse<()> {
    let event: EventPayload = slice.json()?;
    let hub = ctx.state::<SseHub>();
    hub.send("updates", SseEvent::new()
        .event("order_update")
        .data(serde_json::to_string(&event)?))
        .await;
    VilResponse::ok(())
}
```

## Topic-Based Routing

Both hubs support topic-based subscription:

```rust
// Subscribe to specific topic
hub.subscribe("orders");
hub.subscribe("payments");

// Broadcast to topic
hub.broadcast("orders", message).await;

// Broadcast to all topics
hub.broadcast_all(message).await;
```

## WsHub vs SseHub

| Feature | WsHub | SseHub |
|---------|-------|--------|
| Direction | Bidirectional | Server-push only |
| Protocol | WebSocket | HTTP SSE |
| Browser | `WebSocket` API | `EventSource` API |
| Use case | Chat, gaming | Notifications, updates |

> Reference: docs/vil/003-VIL-Developer_Guide-Server-Framework.md
