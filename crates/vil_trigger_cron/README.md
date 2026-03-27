# vil_trigger_cron

VIL Phase 3 cron / scheduled interval trigger.

Fires `TriggerEvent` descriptors on a cron schedule using the `cron` crate.
Supports 5-field and 6-field (leading seconds) cron expressions.

## Tri-Lane Mapping

| Lane    | Direction           | Content                    |
|---------|---------------------|----------------------------|
| Trigger | Outbound → Pipeline | `TriggerEvent` descriptor  |
| Data    | N/A                 | No payload for cron events |
| Control | Inbound ← Pipeline  | Pause / Resume / Stop      |

## Boundary Classification

| Path                       | Strategy               |
|----------------------------|------------------------|
| `TriggerEvent` on Trigger Lane | Copy (flat struct)   |
| Schedule expression        | Hash via `register_str()` |
| Config at startup          | Copy (External layout) |

## Semantic Log

Every fire emits `mq_log!` with `MqPayload`. No `println!`, `tracing::info!`,
or other non-VIL logging is used.

## YAML Configuration Example

```yaml
triggers:
  - kind: cron
    id: daily-report
    schedule: "0 30 6 * * *"   # 06:30 every day (6-field, leading seconds)
    missed_fire: fire_immediately
```
