# 506 — VIL Log: All 7 Structured Event Categories

Demonstrates every semantic log category in `vil_log` with realistic production data.

## Run

```bash
cargo run -p example-506-villog-structured-events
```

## Categories covered

| Macro | Category | What it models |
|-------|----------|----------------|
| `access_log!` | HTTP Access | Incoming API request to POST /api/v1/orders |
| `app_log!` | Application | Order lifecycle: create → payment → ship → SLA breach |
| `ai_log!` | AI Inference | GPT-4o-mini chat completion + semantic cache hit |
| `db_log!` | Database | INSERT order row + slow SELECT with FOR UPDATE |
| `mq_log!` | Message Queue | Kafka publish (lz4) + DLQ after 5 retries |
| `system_log!` | System | Normal metrics snapshot + high-CPU warning |
| `security_log!` | Security | Successful TOTP login + brute-force anomaly deny |

## Notes

All payload structs use `..Default::default()` for unset fields — safe because
`Default` is implemented as `zeroed()` for all `#[repr(C)]` payload types.
