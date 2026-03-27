# vil_trigger_iot — VIL Phase 3 IoT MQTT Trigger

MQTT subscription-based IoT device event trigger for VIL Phase 3.

## Features

- Uses `rumqttc` AsyncClient directly — thin, no extra layers
- Subscribes to configurable topic (supports MQTT wildcards)
- Emits `mq_log!` on every PUBLISH with timing and payload size
- Plain `IotFault` enum — no thiserror, no String fields
- `register_str()` used for all hash fields

## Tri-Lane Mapping

| Lane | Direction | Content |
|------|-----------|---------|
| Trigger | Outbound → Pipeline | TriggerEvent (kind=iot) |
| Data | Outbound → Pipeline | MQTT payload bytes |
| Control | Inbound ← Pipeline | Pause / Resume / Stop |

## Log Emit Table

| Operation | op_type | Macro |
|-----------|---------|-------|
| MQTT PUBLISH received | 1 (consume) | mq_log! |
