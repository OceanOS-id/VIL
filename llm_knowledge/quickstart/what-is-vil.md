# What is VIL?

VIL is a process-oriented language and framework hosted on Rust for building zero-copy, high-performance distributed systems. It combines a semantic language layer (compiler, IR, macros, codegen) with a server framework (VilApp, ServiceProcess, Tri-Lane mesh).

## At a Glance

| Metric | Value |
|--------|-------|
| Rust crates | 130+ |
| Examples | 83 (5 tiers) |
| SDK languages | 9 (Rust + Python, Go, Java, TypeScript, C#, Kotlin, Swift, Zig) |
| Passing tests | 1,425+ |
| Protocols | 7 (REST, SSE, WebSocket, gRPC, Kafka, MQTT, NATS) |
| Execution modes | 3 (Native, WASM, Sidecar) |
| SSE dialects | 5 (OpenAI, Anthropic, Ollama, Cohere, Gemini) |
| Config profiles | 3 (dev, staging, prod) |
| VX_APP throughput | ~41,000 req/s |
| Tri-Lane messaging | 1.9M msg/s |
| Logging | vil_log: 7 semantic types, ~130ns hot path, 4-6x faster than tracing |
| Connectors | Storage (S3/GCS/Azure), DB (Mongo/ClickHouse/Dynamo/Cassandra/Neo4j/Elastic/TS), MQ (RabbitMQ/SQS/Pulsar/PubSub), Protocols (SOAP/OPC-UA/Modbus/WS) |
| Trigger types | 8 (Cron, Filesystem, CDC, Email, IoT, EVM, Webhook, Manual) |

## Three Application Patterns

### 1. VX_APP (Server)
Process-oriented server with SHM zero-copy IPC (~1-5us per hop).

```rust
VilApp::new("my-server")
    .port(8080)
    .service(ServiceProcess::new("api")
        .endpoint(Method::GET, "/", get(handler)))
    .run().await;
```

### 2. SDK_PIPELINE (Streaming)
HTTP streaming pipeline with SSE/NDJSON via `vil_workflow!`.

```rust
let (_ir, handles) = vil_workflow! {
    name: "Gateway",
    instances: [ sink, source ],
    routes: [ sink.out -> source.in (LoanWrite) ]
};
```

### 3. Multi-Pipeline
Multiple `vil_workflow!` pipelines sharing a common `ExchangeHeap`.

## Key Types

| Type | Purpose |
|------|---------|
| `ShmSlice` | Zero-copy body extraction from HTTP requests |
| `ServiceCtx` | Typed state access with Tri-Lane metadata |
| `VilResponse<T>` | SIMD-accelerated JSON response envelope |
| `VilError` | RFC 7807 structured error with status mapping |
| `ShmToken` | 32-byte fixed-size token for SHM transport |
| `GenericToken` | In-memory token for simple pipelines |

## Example Tiers

| Tier | Range | Focus |
|------|-------|-------|
| 1 | 001-029 | Server, CRUD, mesh, SSE, VilServer, SSE Hub, Macro Demo |
| 2 | 101-105 | Multi-pipeline: fan-out, fan-in, diamond |
| 3 | 201-205 | LLM integration |
| 4 | 301-305 | RAG pipelines |
| 5 | 401-405 | Agent patterns |

## Semantic Macros

```rust
#[vil_state]    // Mutable state -> Data Lane
#[vil_event]    // Immutable log -> Data/Control Lane
#[vil_fault]    // Structured error -> Control Lane
#[vil_decision] // Routing logic -> Trigger Lane
```

## Logging (vil_log)

Semantic log system with 7 types auto-emitted from VIL macros and plugins.

```rust
// Production: init_logging → SPSC ring (~130ns)
init_logging(LogConfig { ring_slots: 1 << 20, ..Default::default() }, StdoutDrain::resolved());

// Dev: skip init_logging → tracing fallback (~800ns, human-readable)
app_log!(Info, "order.created", { order_id: 123u64 });
```

See [logging/vil-log.md](../logging/vil-log.md) for full reference.

## Connectors

Native connectors (not integrations) with automatic #[connector_fault/event/state] instrumentation:
- **Storage**: S3, GCS, Azure Blob, MinIO — see [connectors/storage.md](../connectors/storage.md)
- **Databases**: MongoDB, ClickHouse, DynamoDB, Cassandra, Neo4j, Elasticsearch, InfluxDB — see [connectors/databases.md](../connectors/databases.md)
- **Messaging**: RabbitMQ, SQS/SNS, Pulsar, Google Pub/Sub — see [connectors/messaging.md](../connectors/messaging.md)
- **Protocols**: SOAP, OPC-UA, Modbus, WebSocket server — see [connectors/protocols.md](../connectors/protocols.md)

## Triggers

8 trigger types via `TriggerSource` trait — see [triggers/overview.md](../triggers/overview.md).

> Reference: docs/vil/001-VIL-Developer_Guide-Overview.md
