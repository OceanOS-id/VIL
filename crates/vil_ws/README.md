# vil_ws — VIL Dedicated WebSocket Server

WebSocket server with room/channel management for VIL Phase 2 protocol crates.

## Features

- Full WebSocket server via `tokio-tungstenite`
- Room/channel management via `RoomManager` (DashMap-backed)
- Automatic `mq_log!` emit on send (op_type=0) and receive (op_type=1)
- VIL-compliant `WsFault` plain enum (no thiserror, no String fields)
- `register_str()` hashes for all topic and room name fields

## Boundary Classification

| Path | Mode | Notes |
|------|------|-------|
| WebSocket wire (TCP+WS frame) | Copy | Network boundary — WS frame serialization |
| Room broadcast (in-process) | mpsc channel | Tokio unbounded channel per client |
| Internal result passing | Copy | Control-weight |

## Tri-Lane Mapping

| Lane | Direction | Content |
|------|-----------|---------|
| Trigger | Inbound → VIL | New WebSocket message arrival |
| Data | Bidirectional | Message payload |
| Control | Bidirectional | Connection errors / room leave signals |

## Log Emit Table

| Operation | op_type | Macro |
|-----------|---------|-------|
| send / broadcast | 0 (publish) | mq_log! |
| receive | 1 (consume) | mq_log! |

## Thread Hint

WsServer spawns 1 accept loop task. Add 1 to `LogConfig.threads`.

## Compliance

- COMPLIANCE.md §8 — `vil_log` dependency, `mq_log!` auto-emit, `register_str()` hashes
- COMPLIANCE.md §4 — `WsFault` plain enum, no `thiserror`, no `String` fields
