# 014 — Kafka Stream Processing

Kafka producer/consumer with KafkaBridge to Tri-Lane SHM, demonstrating consume-process-produce pattern using in-memory stubs.

| Property | Value |
|----------|-------|
| **Pattern** | VX_APP |
| **Token** | N/A |
| **Body** | ShmSlice (zero-copy) |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A |

## Architecture

```
GET /api/kafka/config, POST /api/kafka/produce, GET /api/kafka/consumer, GET /api/kafka/bridge
```

## Key VIL Features Used

- `KafkaProducer with key-based partitioning`
- `KafkaBridge to Tri-Lane SHM zero-copy`
- `ShmSlice for message body`
- `ServiceCtx with typed state`
- `VilModel + VilResponse`

## Run

```bash
cargo run -p basic-usage-kafka-stream
```

## Test

```bash
curl http://localhost:8080/api/kafka/config
curl -X POST http://localhost:8080/api/kafka/produce -H 'Content-Type: application/json' -d '{"topic":"events.orders","key":"order-42","payload":{"action":"created"}}'
```
