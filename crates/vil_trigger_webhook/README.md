# vil_trigger_webhook — VIL Phase 3 HTTP Webhook Trigger

HTTP webhook receiver trigger with HMAC-SHA256 verification for VIL Phase 3.

## Features

- axum HTTP server — POST to configurable path
- HMAC-SHA256 verification (`x-hub-signature-256` header, GitHub/Stripe/Slack compatible)
- Emits `mq_log!` on every verified webhook with timing and body size
- Plain `WebhookFault` enum — no thiserror, no String fields
- `register_str()` used for all hash fields

## Tri-Lane Mapping

| Lane | Direction | Content |
|------|-----------|---------|
| Trigger | Outbound → Pipeline | TriggerEvent (kind=webhook) |
| Data | Outbound → Pipeline | Webhook body bytes |
| Control | Inbound ← Pipeline | Pause / Resume / Stop |

## Log Emit Table

| Operation | op_type | Macro |
|-----------|---------|-------|
| Webhook POST received + verified | 1 (consume) | mq_log! |
