# VIL Roadmap

> Last updated: 2026-03-27

## Current State (v0.1.0)

VIL ships with **102 crates** and **63 examples** covering:

### Core Runtime & Compiler
vil_types, vil_shm, vil_queue, vil_registry, vil_rt, vil_obs, vil_net, vil_engine, vil_tensor_shm, vil_consensus, vil_operator, vil_ir, vil_diag, vil_validate, vil_macros, vil_codegen_rust, vil_codegen_c, vil_ai_compiler

### Server
vil_server_core, vil_server_web, vil_server_config, vil_server_mesh, vil_server_auth, vil_server_db, vil_server_test, vil_server_macros, vil_server, vil_server_format, vil_new_http

### Database
- **SQL**: PostgreSQL, MySQL, SQLite (via sqlx + SeaORM)
- **Cache**: Redis, SHM-backed cache
- **Semantic**: Provider-neutral compile-time DB IR

### Message Queue
- Kafka, MQTT, NATS (with JetStream + KV Store)

### Protocol
- gRPC (tonic), GraphQL (auto-generated CRUD), HTTP/Axum, JSON (SIMD zero-copy), Protobuf (content negotiation)

### AI/ML (35+ crates)
LLM (multi-provider), RAG, Agent (ReAct), Embedder, Inference Server, VectorDB (native HNSW), Guardrails, Audio, Vision, GraphRAG, Reranker, Multimodal Fusion, Federated RAG, Private RAG, Real-Time RAG, Streaming RAG, LLM Cache, LLM Proxy, AI Gateway, Semantic Router, Prompt Optimizer, Prompt Shield, Output Parser, Memory Graph, Multi-Agent, Model Registry, Model Serving, Quantized Runtime, Speculative Decoding, Tokenizer, Edge Inference, Context Optimizer, AI Trace

### Data Processing
Crawler, Chunker (SIMD), Doc Parser, Doc Extract, Doc Layout, Synthetic Data Generator, RLHF/DPO Pipeline, Data Prep, Index Updater

### Tooling
CLI (`vil init` — 5 languages, 8 templates), SDK, Plugin SDK, LSP, Visualization, Sidecar Protocol

### Scripting
JavaScript (sandboxed), Lua (sandboxed)

### SDK / Transpile Languages
Rust (native), Python, Go, Java, TypeScript

---

## Phase 0 — Q2 2026: VIL Semantic Log System (`vil_log`) ✅ COMPLETED

**Prerequisite for all other phases.**

### Results
- 7 semantic log types, 8 drain backends, auto-sized striped SPSC rings
- Auto-emit from `#[vil_handler]`, `vil_db_*`, `vil_llm`, `vil_mq_*`, `vil_rt`
- 8 examples (501-508), README, full benchmark suite

### Benchmark (actual, single-thread)
| Log Type | ns/event | vs tracing |
|----------|----------|------------|
| Flat types (access, ai, db, mq, system, security) | 130-178 | **4.5-6.2x faster** |
| app_log! (flat struct) | 133 | **6.1x faster** |
| app_log! (dynamic MsgPack) | 390 | **2.1x faster** |
| tracing (fmt + NonBlocking) | 810 | baseline |

### Multi-thread (striped rings, `threads: 8`)
| Threads | VIL access_log! | vs tracing |
|---------|-----------------|------------|
| 1-2 | 7-10 M/s | **2.9-3.8x faster** |
| 4 | 10.5 M/s | **2.0x faster** |
| 8 | 6.3 M/s | **1.0x (parity)** |

---

## Phase 1 — Q3 2026: Storage & Database Expansion ✅ COMPLETED

### Object Storage
- [x] MinIO / S3-compatible (`vil_storage_s3`)
- [x] Google Cloud Storage (`vil_storage_gcs`)
- [x] Azure Blob Storage (`vil_storage_azure`)

### Database
- [x] MongoDB (`vil_db_mongo`) — document store
- [x] ClickHouse (`vil_db_clickhouse`) — OLAP / analytics
- [x] DynamoDB (`vil_db_dynamodb`) — AWS managed KV
- [x] Cassandra / ScyllaDB (`vil_db_cassandra`) — wide-column distributed
- [x] InfluxDB / TimescaleDB (`vil_db_timeseries`) — time-series
- [x] Neo4j (`vil_db_neo4j`) — graph database, complement GraphRAG
- [x] Elasticsearch / OpenSearch (`vil_db_elastic`) — full-text search

All 10 crates: `vil_log` integrated, `db_log!` auto-emit on every operation, COMPLIANCE.md §8 verified.

---

## Phase 2 — Q4 2026: Connector & Message Queue Expansion ✅ COMPLETED

### Message Queue
- [x] RabbitMQ (`vil_mq_rabbitmq`) — AMQP via lapin
- [x] Apache Pulsar (`vil_mq_pulsar`) — pulsar crate
- [x] AWS SQS/SNS (`vil_mq_sqs`) — aws-sdk-sqs/sns
- [x] Google Pub/Sub (`vil_mq_pubsub`) — google-cloud-pubsub
- [ ] Azure Service Bus (`vil_mq_azure_sb`) — deferred
- [ ] Apache Flink bridge (`vil_mq_flink`) — deferred

### Protocol
- [x] SOAP/WSDL (`vil_soap`) — quick-xml + reqwest
- [x] OPC-UA (`vil_opcua`) — opcua client
- [x] Modbus (`vil_modbus`) — tokio-modbus
- [ ] AMQP 1.0 (`vil_amqp`) — deferred
- [x] WebSocket server (`vil_ws`) — tokio-tungstenite
- [x] Server-Sent Events (`vil_sse`) — tokio channels

All 9 crates: `vil_log` integrated, `mq_log!`/`db_log!` auto-emit, COMPLIANCE.md §8 verified.

---

## Phase 3 — Q1 2027: Trigger & Event Source ✅ COMPLETED

- [x] Trigger core (`vil_trigger_core`) — TriggerSource trait, EventCallback, TriggerEvent
- [x] Cron / Schedule trigger (`vil_trigger_cron`) — cron expressions, missed-fire policy
- [x] File / S3 watcher trigger (`vil_trigger_fs`) — notify crate, glob patterns, debounce
- [x] Database CDC trigger (`vil_trigger_cdc`) — PostgreSQL logical replication
- [x] Email trigger (`vil_trigger_email`) — IMAP IDLE via async-imap
- [x] IoT device event trigger (`vil_trigger_iot`) — MQTT via rumqttc
- [x] Blockchain event trigger (`vil_trigger_evm`) — alloy, EVM log subscription
- [x] Webhook receiver (`vil_trigger_webhook`) — axum + HMAC verification

All 8 crates: `vil_log` + `mq_log!` auto-emit, `TriggerSource` trait, COMPLIANCE.md §8 verified.

---

## Phase 4 — Q2 2027: SDK & Platform ✅ COMPLETED

### SDK Languages (now 9 total: Rust + 8 transpile)
- [x] C# / .NET (`vil init --lang csharp`) — .csproj + app.vil.cs
- [x] Kotlin (`vil init --lang kotlin`) — build.gradle.kts + app.vil.kt
- [x] Swift (`vil init --lang swift`) — Package.swift + app.vil.swift
- [x] Zig (`vil init --lang zig`) — build.zig + app.vil.zig

### Platform
- [x] crates.io metadata — repository, homepage, documentation, keywords, categories
- [ ] VIL Cloud — managed deployment (SaaS) — deferred
- [ ] VIL Marketplace — community connectors & templates — deferred
- [ ] VIL Playground — browser-based WASM sandbox — deferred

---

## Phase 5a — H2 2027: Open-Source Enterprise ✅ COMPLETED

- [x] OpenTelemetry export (`vil_otel`) — OTLP gRPC/HTTP, metrics + traces bridge
- [x] Grafana dashboard templates (6 dashboards + 3 alert rules)
- [x] Edge deployment (`vil_edge_deploy`) — ARM64, ARMv7, RISC-V profiles

## Phase 5b — Commercial (separate repo)

- [ ] Multi-tenancy & namespace isolation
- [ ] Compliance connectors — audit trail, GDPR tooling
- [ ] Plugin marketplace with community review system
