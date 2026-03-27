# vil_trigger_evm — VIL Phase 3 EVM Blockchain Event Trigger

Ethereum JSON-RPC log subscription trigger for VIL Phase 3.

## Features

- Uses `alloy` 0.15 — modern Rust Ethereum library
- `eth_subscribe` logs with address + topic0 filter
- Emits `mq_log!` on every log with timing, block number, contract hash
- Plain `EvmFault` enum — no thiserror, no String fields
- `register_str()` used for all hash fields

## Tri-Lane Mapping

| Lane | Direction | Content |
|------|-----------|---------|
| Trigger | Outbound → Pipeline | TriggerEvent (kind=evm) |
| Data | Outbound → Pipeline | Log data bytes |
| Control | Inbound ← Pipeline | Pause / Resume / Stop |

## Log Emit Table

| Operation | op_type | Macro |
|-----------|---------|-------|
| Contract log received | 1 (consume) | mq_log! |
