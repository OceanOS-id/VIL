# vil_trigger_cdc — VIL Phase 3 CDC Trigger

PostgreSQL logical replication CDC trigger for VIL Phase 3.

## Features

- Connects via `tokio-postgres` in logical replication mode
- Consumes `INSERT` / `UPDATE` / `DELETE` events from a pgoutput publication
- Emits `mq_log!` on every trigger fire with timing and table hash
- Plain `CdcFault` enum — no thiserror, no String fields
- `register_str()` used for all hash fields

## Tri-Lane Mapping

| Lane | Direction | Content |
|------|-----------|---------|
| Trigger | Outbound → Pipeline | TriggerEvent (kind=cdc) |
| Data | Outbound → Pipeline | Row before/after (future) |
| Control | Inbound ← Pipeline | Pause / Resume / Stop |

## Log Emit Table

| Operation | op_type | Macro |
|-----------|---------|-------|
| INSERT fire | 0 (publish) | mq_log! |
| UPDATE fire | 1 (consume) | mq_log! |
| DELETE fire | 2 (ack)     | mq_log! |
