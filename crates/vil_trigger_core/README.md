# vil_trigger_core — VIL Phase 3 Shared Trigger Infrastructure

Shared trait, types, and helpers used by all VIL Phase 3 trigger crates.

## Features

- `TriggerSource` async trait — implemented by every trigger crate
- `TriggerEvent` — fixed-size Trigger Lane descriptor (no heap)
- `TriggerFault` — plain enum fault type (no thiserror, no String fields)
- `create_trigger()` — Arc-wrap any TriggerSource for ServiceProcess use

## Boundary Classification

| Path | Mode | Notes |
|------|------|-------|
| TriggerEvent emission | Copy | Fixed-size struct, no heap |
| TriggerFault propagation | Copy | Plain enum, all u32 fields |

## Tri-Lane Mapping

| Lane | Direction | Content |
|------|-----------|---------|
| Trigger | Outbound → Pipeline | TriggerEvent descriptor |
| Data | Outbound → Pipeline | Event payload (crate-specific) |
| Control | Inbound ← Pipeline | Pause / Resume / Stop |
