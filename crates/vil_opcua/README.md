# vil_opcua — VIL OPC-UA Client

Industrial OPC-UA protocol client for VIL Phase 2 protocol crates.

## Features

- Connect, read_node, write_node, subscribe via `opcua` crate
- Automatic `db_log!` emit on every operation with wall-clock timing
- VIL-compliant `OpcUaFault` plain enum (no thiserror, no String fields)
- `register_str()` hashes for all node ID and endpoint fields

## Boundary Classification

| Path | Mode | Notes |
|------|------|-------|
| OPC-UA wire (TCP) | Copy | Network boundary — OPC-UA binary protocol |
| Internal result passing | Copy | Control-weight, not hot-path |

## Tri-Lane Mapping

| Lane | Direction | Content |
|------|-----------|---------|
| Trigger | Inbound → VIL | Read/write request descriptor |
| Data | Outbound ← VIL | Node value result |
| Control | Bidirectional | Session errors / reconnect signals |

## Log Emit Table

| Operation | op_type | Macro |
|-----------|---------|-------|
| read_node | 0 (SELECT) | db_log! |
| write_node | 2 (UPDATE) | db_log! |
| subscribe | 4 (CALL) | db_log! |

## Thread Hint

OPC-UA session management spawns internal threads.
Add 2 to `LogConfig.threads` for optimal log ring sizing.

## Compliance

- COMPLIANCE.md §8 — `vil_log` dependency, `db_log!` auto-emit, `register_str()` hashes
- COMPLIANCE.md §4 — `OpcUaFault` plain enum, no `thiserror`, no `String` fields
