<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/assets/vil-logo-dark.svg"/>
    <source media="(prefers-color-scheme: light)" srcset="docs/assets/vil-logo-light.svg"/>
    <img src="docs/assets/vil-logo-dark.svg" alt="VIL Logo" width="200"/>
  </picture>
</p>

<h1 align="center">VIL — Vastar Intermediate Language</h1>

<p align="center">
  A <strong>process-oriented language and framework</strong> hosted on Rust for building zero-copy, high-performance distributed systems.
</p>

<p align="center">
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-blue" alt="License"></a>
  <img src="https://img.shields.io/badge/crates-171-green" alt="Crates">
  <img src="https://img.shields.io/badge/examples-234-orange" alt="Examples">
  <img src="https://img.shields.io/badge/built--in_FaaS-20-cyan" alt="FaaS">
  <img src="https://img.shields.io/badge/v0.3.0-latest-brightgreen" alt="Version">
</p>

VIL combines a **semantic language layer** (compiler, IR, macros, codegen) with a **server framework** (VilApp, ServiceProcess, Tri-Lane mesh) — generating all plumbing so developers write only business logic and intent.

**v0.3.0:** Two development patterns (Standard + Workflow), WASM at 83K req/s, Sidecar at 59K req/s, 20 built-in FaaS functions.

## Two Patterns, One Runtime

### Standard Pattern — Imperative Rust

```rust
use vil_server::prelude::*;

#[tokio::main]
async fn main() {
    VilApp::new("my-service")
        .service(ServiceProcess::new("handler")
            .endpoint(Method::POST, "/api/process", post(my_handler)))
        .run("0.0.0.0:8080").await;
}
```

### Workflow Pattern — Declarative YAML + Any Language

```rust
#[tokio::main]
async fn main() {
    vil_vwfd::app("workflows/", 8080)
        .native("validate", |input| Ok(json!({"ok": true})))
        .wasm("pricing", "modules/pricing.wasm")
        .sidecar("scorer", "python3 scorer.py")
        .run().await;
}
```

```yaml
# workflows/order.yaml
- id: process
  activity_type: Transform
  input_mappings:
    - target: order_id
      source: { language: vil-expr, source: 'uuid_v4()' }
    - target: total
      source: { language: vil-expr, source: 'mean(trigger_payload.body.prices)' }
    - target: email_hash
      source: { language: vil-expr, source: 'sha256(trigger_payload.body.email)' }
```

## 3 Execution Modes

| Mode | Throughput | Latency | Use Case |
|------|-----------|---------|----------|
| **Native Rust** | 97K req/s | 0.5 ms | Core logic, maximum performance |
| **WASM Sandbox** | 83K req/s | 0.5 ms | Hot-deploy, plugins, 6 WASM languages |
| **Sidecar** | 59K req/s | 0.5 ms | Python, Node.js, PHP, Lua, C# — 9 languages |

WASM is **WASI-compliant** with PoolingAllocator + InstancePre + WasmWorkerPool.
Sidecar uses **keep-alive process pool x4** with line-delimited JSON.

## 20 Built-in FaaS Functions

Zero custom code — call directly from YAML expressions:

```yaml
# Security
source: { language: vil-expr, source: 'sha256(data)' }
source: { language: vil-expr, source: 'jwt_sign(payload, "secret")' }

# Identity
source: { language: vil-expr, source: 'uuid_v4()' }
source: { language: vil-expr, source: 'parse_phone("+6281234567890", "ID")' }

# Analytics
source: { language: vil-expr, source: 'is_anomaly(amount, history, "zscore", 2.0)' }
source: { language: vil-expr, source: 'geo_distance(-6.2, 106.8, -7.8, 110.4, "km")' }

# Data
source: { language: vil-expr, source: 'parse_csv(data, ",", true)' }
source: { language: vil-expr, source: 'render_template("Hello {{name}}", data)' }
```

**All 20:** sha256, md5, hmac_sha256, aes_encrypt, aes_decrypt, jwt_sign, jwt_verify, uuid_v4, uuid_v7, ulid, nanoid, parse_date, format_date, now, age, duration, parse_csv, parse_xml, xpath, regex_match, regex_extract, regex_replace, parse_phone, validate_email, validate_schema, mask_pii, reshape, render_template, mean, median, stdev, percentile, variance, is_anomaly, send_email, send_webhook, geo_distance

```toml
vil_vwfd = { features = ["faas-full"] }  # enable all 20
```

## Performance

> Intel i9-11900F (8C/16T), 32GB RAM, Ubuntu 22.04, vastar bench (warmed, c=10)

| Mode | Language | req/s | vs Native |
|------|----------|-------|-----------|
| Native | Rust | 97,331 | 1.00x |
| WASM | Rust | 87,417 | 0.90x |
| WASM | AssemblyScript | 83,373 | 0.86x |
| WASM | C | 80,540 | 0.83x |
| Sidecar | PHP | 59,101 | 0.61x |
| Sidecar | Lua | 58,365 | 0.60x |
| WASM | Java | 57,246 | 0.59x |
| Sidecar | Node.js | 35,480 | 0.36x |
| Sidecar | C# | 26,251 | 0.27x |

At c=200 concurrency, WASM **exceeds** NativeCode (25K vs 24.6K) due to dedicated worker threads.

## What's Inside

| Layer | Crates | Purpose |
|-------|--------|---------|
| **Runtime** | vil_types, vil_shm, vil_queue, vil_rt | Zero-copy SHM, SPSC queues |
| **Compiler** | vil_ir, vil_expr, vil_rules, vil_macros | VIL Expression evaluator, YAML rules, codegen |
| **Server** | vil_server (9 crates) | VilApp, Tri-Lane mesh, auth, config |
| **Workflow** | vil_vwfd, vil_vwfd_macros | VWFD compiler + executor, VwfdApp builder |
| **Execution** | vil_capsule, vil_sidecar | WASM sandbox (wasmtime), Sidecar SDK (UDS+SHM) |
| **Connectors** | 30 crates | DB (10), MQ (7), Storage (3), Protocol (6), Codec (3), SFTP |
| **Triggers** | 13 crates | Webhook, Cron, Kafka, S3, SFTP, CDC, FS, MQTT, Email, EVM, gRPC, DB poll |
| **Built-in FaaS** | 20 crates | Security, Date, Parsing, Text, Transform, Stats, Notification, Geo |
| **AI Plugins** | 51 crates | LLM, RAG, Agent, embeddings, vector DB |
| **Observability** | vil_log, vil_observer, vil_otel | Semantic log, dashboard, Prometheus, OpenTelemetry |

**171 crates** | **234 examples** | **20 built-in FaaS** | **13 triggers** | **30 connectors**

## Examples (9 Tiers)

| Tier | Count | Highlights |
|------|-------|------------|
| **Basic** (001-047) | 47 | HTTP, WebSocket, GraphQL, SSE, WASM, Sidecar, Auth |
| **Pipeline** (101-108) | 10 | Fan-out, fan-in, diamond, DAG, SSE, traced |
| **LLM** (201-206) | 6 | Chat, multi-model, streaming, tools, decision routing |
| **RAG** (301-308) | 8 | Vector search, hybrid, guardrail, citation, full pipeline |
| **Agent** (401-407) | 7 | Calculator, researcher, multi-agent orchestration |
| **VIL Log** (501-509) | 9 | Stdout, file, multi-drain, benchmark, tracing bridge |
| **Database** (601-611) | 11 | SQLite, MongoDB, S3, TimeSeries, VilORM, multi-tenant |
| **MQ/Protocol** (701-706) | 6 | RabbitMQ, gRPC, SOAP, Modbus, Pulsar |
| **FaaS Demo** (901-905) | 5 | KYC, Data Pipeline, Secure API, Financial, Notification |

Each example has **two versions**: Standard (`src/main.rs`) and Workflow (`vwfd/`).

```bash
# Standard
cargo run --release -p vil-basic-hello-server

# Workflow (VWFD)
cargo run --release -p vil-vwfd-currency-exchange
```

## Quick Start

```bash
# Install CLI
cargo install vil-cli

# Create project
vil init my-api --template vwfd
cd my-api

# Run
cargo run --release
curl http://localhost:8080/api/hello
```

## Connectors & Triggers

### Connectors (30)

**Database:** PostgreSQL, MySQL, SQLite, Redis, MongoDB, Cassandra, ClickHouse, DynamoDB, Elasticsearch, Neo4j, TimeSeries
**Message Queue:** NATS, Kafka, MQTT, RabbitMQ, Pulsar, Google Pub/Sub, AWS SQS
**Storage:** S3/MinIO/R2, Google Cloud Storage, Azure Blob
**Protocol:** HTTP/SSE, SFTP, SOAP/WSDL, WebSocket, Modbus, OPC-UA
**Codec:** ISO 8583, MessagePack, Protobuf

### Triggers (13)

Webhook, Cron, Kafka Consumer, S3 Bucket Event, SFTP Directory, PostgreSQL CDC, DB Poll, Filesystem Watch, MQTT/IoT, Email IMAP, EVM Blockchain, gRPC Stream

## The 10 Immutable Principles

1. **Everything is a Process** — identity, ports, failure domain
2. **Zero-Copy is a Contract** — VASI/PodLike, ExchangeHeap
3. **IR is the Truth** — macros are frontend, vil_ir is source of truth
4. **Generated Plumbing** — developers never write queue push/pop
5. **Safety Through Semantics** — type system + IR + validation passes
6. **Three Layout Profiles** — Flat, Relative, External
7. **Semantic Message Types** — `#[vil_state/event/fault/decision]`
8. **Tri-Lane Protocol** — Trigger / Data / Control (no head-of-line blocking)
9. **Ownership Transfer Model** — LoanWrite, LoanRead, PublishOffset, Copy
10. **Observable by Design** — `#[trace_hop]`, metrics auto-generated

## Documentation

- **Website:** [vastar.id/products/vil](https://vastar.id/products/vil)
- **Docs:** [vastar.id/docs/vil](https://vastar.id/docs/vil)
- **Architecture:** [docs/ARCHITECTURE_OVERVIEW.md](docs/ARCHITECTURE_OVERVIEW.md)
- **VIL Guide (11 parts):** [docs/vil/](docs/vil/)
- **VWFD YAML Reference:** [vastar.id/docs/vil/reference/vwfd-yaml](https://vastar.id/docs/vil/reference/vwfd-yaml)
- **FaaS Functions:** [vastar.id/docs/vil/reference/faas-functions](https://vastar.id/docs/vil/reference/faas-functions)

## Editor Support (in-development)

`vil-lsp` provides diagnostics, completions, and hover for VIL macros alongside `rust-analyzer`.

| Editor | Setup | Status |
|--------|-------|--------|
| VS Code | [editors/vscode/](editors/vscode/) | In development |
| Zed | [editors/zed/](editors/zed/) | In development |
| Helix | [editors/helix/](editors/helix/) | In development |
| JetBrains | [editors/jetbrains/](editors/jetbrains/) | In development |

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.

## Links

- **Repository:** [github.com/OceanOS-id/VIL](https://github.com/OceanOS-id/VIL)
- **Website:** [vastar.id/products/vil](https://vastar.id/products/vil)
- **Community Simulators:**
  - [AI Endpoint Simulator](https://github.com/Vastar-AI/ai-endpoint-simulator)
  - [Credit Data Simulator](https://github.com/Vastar-AI/credit-data-simulator)
