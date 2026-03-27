# 010 — WebSocket Chat Room

VX_APP with WebSocket broadcast chat using WsHub and VilWsEvent typed messages.

| Property | Value |
|----------|-------|
| **Pattern** | VX_APP |
| **Token** | N/A |
| **Body** | N/A (WebSocket frames) |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A |

## Architecture

```
GET / (HTML client), GET /ws (WebSocket), GET /stats (client count)
```

## Key VIL Features Used

- `WsHub for topic-based broadcast`
- `VilWsEvent derive macro with #[ws_event(topic)]`
- `ServiceProcess + VilApp process-oriented architecture`
- `VilResponse for REST stats endpoint`
- `VilModel derive for ChatStats`

## Run

```bash
cargo run -p basic-usage-websocket-chat
```

## Test

```bash
websocat ws://localhost:8080/api/chat/ws
curl http://localhost:8080/api/chat/stats
```
