# vil_trigger_email — VIL Phase 3 IMAP Email Trigger

IMAP IDLE push-based email trigger for VIL Phase 3.

## Features

- IMAP IDLE — server push, no polling
- TLS via `async-native-tls`
- Emits `mq_log!` on every new message with timing and folder hash
- Plain `EmailFault` enum — no thiserror, no String fields
- `register_str()` used for all hash fields

## Tri-Lane Mapping

| Lane | Direction | Content |
|------|-----------|---------|
| Trigger | Outbound → Pipeline | TriggerEvent (kind=email) |
| Data | Outbound → Pipeline | Message metadata |
| Control | Inbound ← Pipeline | Pause / Resume / Stop |

## Log Emit Table

| Operation | op_type | Macro |
|-----------|---------|-------|
| New mail IDLE notify | 1 (consume) | mq_log! |
