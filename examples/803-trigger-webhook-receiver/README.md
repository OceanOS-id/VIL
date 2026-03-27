# 803-trigger-webhook-receiver

HTTP webhook receiver with HMAC-SHA256 verification.

## What it shows

- `WebhookTrigger::new()` binding an HTTP listener on port 8090
- `TriggerSource::start()` accepting POST requests
- HMAC-SHA256 signature verification via `X-Hub-Signature-256` header
- `mq_log!` auto-emitted by `vil_trigger_webhook` on every valid delivery
- `StdoutDrain::resolved()` output format

## No external services required

The example starts a local HTTP server and sends test requests to itself
using `reqwest`. No Docker needed.

## Run

```bash
cargo run -p example-803-trigger-webhook-receiver
```

## Production HMAC usage

Set `SECRET` to a non-empty string and sign payloads with HMAC-SHA256:

```
X-Hub-Signature-256: sha256=<hex-encoded-hmac>
```

Add `hmac = "0.12"`, `sha2 = "0.10"`, and `hex = "0.4"` to your Cargo.toml
to compute signatures in Rust.
