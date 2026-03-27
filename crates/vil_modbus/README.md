# vil_modbus — VIL Modbus TCP/RTU Client

Industrial Modbus TCP/RTU protocol client for VIL Phase 2 protocol crates.

## Features

- read_coils, read_registers, write_coil, write_register via `tokio-modbus`
- Automatic `db_log!` emit on every operation with wall-clock timing
- VIL-compliant `ModbusFault` plain enum (no thiserror, no String fields)
- `register_str()` hashes for all address and host fields

## Boundary Classification

| Path | Mode | Notes |
|------|------|-------|
| Modbus TCP wire | Copy | Network boundary — Modbus binary protocol |
| Internal result passing | Copy | Control-weight coil/register values |

## Tri-Lane Mapping

| Lane | Direction | Content |
|------|-----------|---------|
| Trigger | Inbound → VIL | Read/write request descriptor |
| Data | Outbound ← VIL | Register/coil values |
| Control | Bidirectional | Connection errors / timeout signals |

## Log Emit Table

| Operation | op_type | Macro |
|-----------|---------|-------|
| read_coils | 0 (SELECT) | db_log! |
| read_registers | 0 (SELECT) | db_log! |
| write_coil | 2 (UPDATE) | db_log! |
| write_register | 2 (UPDATE) | db_log! |

## Thread Hint

ModbusClient is async on the tokio runtime. No extra log threads.
Add 0 to `LogConfig.threads`.

## Compliance

- COMPLIANCE.md §8 — `vil_log` dependency, `db_log!` auto-emit, `register_str()` hashes
- COMPLIANCE.md §4 — `ModbusFault` plain enum, no `thiserror`, no `String` fields
