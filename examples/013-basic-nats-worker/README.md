# 013 — NATS Pub/Sub + JetStream

NATS Core pub/sub, JetStream persistent streaming, KV store, and NatsBridge to Tri-Lane SHM using in-memory stubs.

| Property | Value |
|----------|-------|
| **Pattern** | VX_APP |
| **Token** | N/A |
| **Body** | ShmSlice (zero-copy) |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A |

## Architecture

```
GET /api/nats/config, POST /api/nats/publish, GET /api/nats/jetstream, GET /api/nats/kv
```

## Key VIL Features Used

- `NatsClient pub/sub with NatsBridge to Tri-Lane SHM`
- `JetStreamClient for persistent streams`
- `KvStore for distributed key-value`
- `ShmSlice for publish body`
- `ServiceCtx state injection`

## Run

```bash
cargo run -p basic-usage-nats-worker
```

## Test

```bash
curl http://localhost:8080/api/nats/config
curl -X POST http://localhost:8080/api/nats/publish -H 'Content-Type: application/json' -d '{"subject":"events.order.created","payload":{"order_id":42}}'
```
