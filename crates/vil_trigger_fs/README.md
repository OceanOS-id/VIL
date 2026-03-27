# vil_trigger_fs

VIL Phase 3 filesystem / directory watcher trigger.

Watches a local path with the `notify` crate (inotify/FSEvents/kqueue) and
fires `TriggerEvent` descriptors on matching create/modify/delete events.

## Tri-Lane Mapping

| Lane    | Direction           | Content                       |
|---------|---------------------|-------------------------------|
| Trigger | Outbound → Pipeline | `TriggerEvent` descriptor     |
| Data    | N/A                 | Path stored as hash (no SHM)  |
| Control | Inbound ← Pipeline  | Pause / Resume / Stop         |

## Boundary Classification

| Path                       | Strategy              |
|----------------------------|-----------------------|
| `TriggerEvent` on Trigger Lane | Copy (flat struct)  |
| File path context          | Hash via `register_str()` |
| Config at startup          | Copy (External layout) |

## Semantic Log

Every fire emits `mq_log!` with `MqPayload`. No `println!`, `tracing::info!`,
or other non-VIL logging is used.

## YAML Configuration Example

```yaml
triggers:
  - kind: fs
    id: csv-watcher
    watch_path: /data/incoming
    pattern: "*.csv"
    debounce_ms: 500
    recursive: false
    events:
      on_create: true
      on_modify: true
      on_delete: false
```
