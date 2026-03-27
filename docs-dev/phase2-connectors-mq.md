# Phase 2 — Q4 2026: Connector & Message Queue Expansion

> **⚠ MANDATORY: Read [COMPLIANCE.md](./COMPLIANCE.md) before implementing any crate in this phase.**
> Every crate must pass the full compliance checklist (P1–P10, testing, docs, pre-merge review).
> Non-compliant crates will be rejected regardless of functionality.

## Objective

Expand VIL's integration surface to cover enterprise messaging systems, industrial protocols, and legacy connectivity. All new MQ crates follow the existing Tri-Lane bridge pattern established by `vil_mq_kafka`, `vil_mq_mqtt`, `vil_mq_nats`.

---

## 1. Message Queue

### 1.1 `vil_mq_rabbitmq` — RabbitMQ (AMQP 0-9-1)

**Priority**: High (most common enterprise MQ)

**Scope**:
- Connection + channel pooling
- Exchange/queue declaration, binding
- Publish with confirm mode
- Consumer with manual/auto ack
- Dead letter exchange (DLX) support
- Tri-Lane bridge: Trigger Lane = consume, Data Lane = payload, Control Lane = ack/nack/reject

**Dependencies**:
- `lapin` crate (async AMQP 0-9-1)
- `vil_types`, `vil_rt`, `vil_obs`

**Implementation Plan**:
```
crates/vil_mq_rabbitmq/
├── src/
│   ├── lib.rs
│   ├── connection.rs   — pool + reconnect logic
│   ├── publisher.rs    — publish with confirm
│   ├── consumer.rs     — consumer with prefetch
│   ├── exchange.rs     — exchange/queue/binding management
│   ├── dlx.rs          — dead letter exchange config
│   ├── bridge.rs       — Tri-Lane bridge adapter
│   └── error.rs
├── tests/
│   └── integration.rs  — Docker RabbitMQ
└── examples/
    ├── rabbitmq_pubsub.rs
    └── rabbitmq_work_queue.rs
```

**Estimated effort**: 3-4 days

---

### 1.2 `vil_mq_pulsar` — Apache Pulsar

**Priority**: Medium

**Scope**:
- Producer (partitioned topics, batching)
- Consumer (exclusive, shared, failover, key_shared subscriptions)
- Schema registry integration
- Pulsar Functions bridge (optional)
- Tri-Lane bridge

**Dependencies**:
- `pulsar` crate
- `vil_types`, `vil_rt`, `vil_obs`

**Implementation Plan**:
```
crates/vil_mq_pulsar/
├── src/
│   ├── lib.rs
│   ├── producer.rs     — batched + partitioned producer
│   ├── consumer.rs     — multi-subscription-type consumer
│   ├── schema.rs       — schema registry bridge
│   ├── bridge.rs       — Tri-Lane bridge
│   └── error.rs
├── tests/
└── examples/
```

**Estimated effort**: 3-4 days

---

### 1.3 `vil_mq_sqs` — AWS SQS/SNS

**Priority**: Medium-High (AWS-heavy deployments)

**Scope**:
- SQS: send, receive, delete, batch operations
- SNS: publish, topic management, subscription
- FIFO queue support (dedup, message groups)
- Long polling consumer
- SNS → SQS fan-out pattern
- Tri-Lane bridge

**Dependencies**:
- `aws-sdk-sqs`, `aws-sdk-sns`
- `vil_types`, `vil_rt`, `vil_obs`

**Implementation Plan**:
```
crates/vil_mq_sqs/
├── src/
│   ├── lib.rs
│   ├── sqs.rs          — SQS client (standard + FIFO)
│   ├── sns.rs          — SNS client
│   ├── fanout.rs       — SNS→SQS pattern helper
│   ├── bridge.rs       — Tri-Lane bridge
│   └── error.rs
├── tests/
│   └── integration.rs  — LocalStack
└── examples/
```

**Testing**: LocalStack for local SQS/SNS emulation.

**Estimated effort**: 3 days

---

### 1.4 `vil_mq_pubsub` — Google Pub/Sub

**Priority**: Medium

**Scope**:
- Publisher with batching + ordering keys
- Subscriber (pull + streaming pull)
- Dead letter topics
- Message filtering
- Tri-Lane bridge

**Dependencies**:
- `google-cloud-pubsub` or gRPC client
- `vil_types`, `vil_rt`, `vil_obs`

**Estimated effort**: 3 days

---

### 1.5 `vil_mq_azure_sb` — Azure Service Bus

**Priority**: Medium

**Scope**:
- Queue + topic/subscription model
- Sessions (ordered processing)
- Scheduled messages
- Dead letter queue
- Tri-Lane bridge

**Dependencies**:
- `azure_messaging_servicebus` or raw AMQP 1.0
- `vil_types`, `vil_rt`, `vil_obs`

**Estimated effort**: 3 days

---

### 1.6 `vil_mq_flink` — Apache Flink Bridge

**Priority**: Low (advanced stream processing)

**Scope**:
- Flink REST API client (job submission, monitoring)
- VIL pipeline → Flink job descriptor conversion
- Result ingestion back into VIL Tri-Lane

**Dependencies**:
- `reqwest` (Flink REST API)
- `vil_types`, `vil_rt`

**Estimated effort**: 5 days (complex integration)

---

## 2. Protocol Connectors

### 2.1 `vil_soap` — SOAP/WSDL

**Priority**: Medium (legacy enterprise — banking, telecom, government)

**Scope**:
- WSDL parser → Rust client codegen
- SOAP envelope builder/parser (XML)
- WS-Security (UsernameToken, X.509)
- MTOM attachment support
- Bridge: wrap SOAP calls as VIL pipeline stages

**Dependencies**:
- `quick-xml` for XML parsing
- `reqwest` for HTTP transport
- `vil_types`, `vil_rt`

**Implementation Plan**:
```
crates/vil_soap/
├── src/
│   ├── lib.rs
│   ├── wsdl.rs         — WSDL parser
│   ├── envelope.rs     — SOAP envelope build/parse
│   ├── security.rs     — WS-Security
│   ├── codegen.rs      — WSDL → Rust client generation
│   ├── client.rs       — runtime SOAP client
│   └── error.rs
├── tests/
└── examples/
    └── soap_legacy_bank.rs
```

**Estimated effort**: 5-7 days (XML complexity)

---

### 2.2 `vil_opcua` — OPC-UA

**Priority**: Medium (industrial IoT — manufacturing, energy, SCADA)

**Scope**:
- OPC-UA client (browse, read, write, subscribe)
- Session management + security policies
- Data change subscription → Tri-Lane Trigger Lane
- Alarm & condition monitoring

**Dependencies**:
- `opcua` crate
- `vil_types`, `vil_rt`, `vil_obs`

**Implementation Plan**:
```
crates/vil_opcua/
├── src/
│   ├── lib.rs
│   ├── client.rs       — OPC-UA client session
│   ├── browse.rs       — node browsing
│   ├── read_write.rs   — variable read/write
│   ├── subscribe.rs    — data change subscription
│   ├── alarm.rs        — alarm & condition
│   ├── bridge.rs       — Tri-Lane bridge
│   └── error.rs
├── tests/
└── examples/
    └── opcua_scada_monitor.rs
```

**Estimated effort**: 5-6 days

---

### 2.3 `vil_modbus` — Modbus TCP/RTU

**Priority**: Medium (industrial — PLCs, sensors, actuators)

**Scope**:
- Modbus TCP client
- Modbus RTU over serial (via tokio-serial)
- Read/write coils, registers
- Polling loop with configurable interval
- Bridge: polled values → Tri-Lane Data Lane

**Dependencies**:
- `tokio-modbus`
- `vil_types`, `vil_rt`

**Estimated effort**: 2-3 days (simpler protocol)

---

### 2.4 `vil_amqp` — AMQP 1.0

**Priority**: Low (distinct from RabbitMQ's AMQP 0-9-1)

**Scope**:
- AMQP 1.0 sender/receiver
- Link management
- Works with Azure Service Bus, Solace, ActiveMQ Artemis

**Dependencies**:
- `fe2o3-amqp` crate
- `vil_types`, `vil_rt`

**Estimated effort**: 3-4 days

---

### 2.5 `vil_ws` — WebSocket Server

**Priority**: Medium

**Scope**:
- Dedicated WebSocket server (not via Axum handler)
- Room/channel management
- Binary + text frame support
- Backpressure handling
- Tri-Lane: incoming frames → Trigger Lane, outgoing → Data Lane

**Dependencies**:
- `tokio-tungstenite`
- `vil_types`, `vil_rt`

**Estimated effort**: 2-3 days

---

### 2.6 `vil_sse` — Server-Sent Events

**Priority**: Medium

**Scope**:
- SSE endpoint builder
- Event ID, retry, named events
- Multi-client fan-out with backpressure
- Heartbeat / keep-alive

**Dependencies**:
- `axum` (SSE response type) or standalone
- `vil_types`, `vil_rt`

**Estimated effort**: 1-2 days (lightweight)

---

## Shared Patterns

All MQ crates must implement:

```rust
#[async_trait]
pub trait MqBridge: Send + Sync {
    /// Connect and start consuming
    async fn start(&self, config: MqConfig) -> Result<()>;

    /// Publish a message
    async fn publish(&self, topic: &str, payload: &[u8], headers: Option<Headers>) -> Result<()>;

    /// Subscribe with Tri-Lane bridge
    async fn subscribe(&self, topic: &str, handler: TriLaneHandler) -> Result<SubscriptionId>;

    /// Graceful shutdown
    async fn stop(&self) -> Result<()>;
}
```

---

## Milestone Checklist

- [ ] `vil_mq_rabbitmq` — implemented + tested with Docker
- [ ] `vil_mq_pulsar` — implemented + tested with Docker
- [ ] `vil_mq_sqs` — implemented + tested with LocalStack
- [ ] `vil_mq_pubsub` — implemented + tested with emulator
- [ ] `vil_mq_azure_sb` — implemented + tested
- [ ] `vil_mq_flink` — implemented + tested
- [ ] `vil_soap` — implemented + tested with mock SOAP server
- [ ] `vil_opcua` — implemented + tested with OPC-UA simulator
- [ ] `vil_modbus` — implemented + tested with Modbus simulator
- [ ] `vil_amqp` — implemented + tested
- [ ] `vil_ws` — implemented + tested
- [ ] `vil_sse` — implemented + tested
- [ ] All MQ crates implement `MqBridge` trait
- [ ] All crates have Tri-Lane bridge integration
- [ ] `vil init` templates updated for new connectors
- [ ] README, examples, benchmarks for all crates
