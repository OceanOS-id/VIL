# VIL Examples

63 production-quality examples organized into 5 tiers, covering every VIL feature.

## Prerequisites

### 1. Rust Toolchain

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable  # requires Rust 1.75+
```

### 2. Infrastructure Services (Docker)

Many examples connect to **real services** — not mocks or simulators. Start all with one command:

```bash
docker compose -f examples/docker-compose.yml up -d
```

This starts:

| Service | Container | Port | Used By |
|---------|-----------|------|---------|
| **PostgreSQL 15** | vil-postgres | 5432 | 012-plugin-database |
| **Redis 7** | vil-redis | 6380 | 012-plugin-database |
| **NATS** (JetStream) | vil-nats | 4222 | 013-nats-worker |
| **Kafka** (Redpanda) | vil-redpanda | 9092 | 014-kafka-stream |
| **MQTT** (Mosquitto) | vil-mosquitto | 1883 | 015-mqtt-iot-gateway |

Connection details:

```
PostgreSQL: postgres://postgres:vilpass@localhost:5432/vil_demo
Redis:      redis://localhost:6380
NATS:       nats://localhost:4222
Kafka:      localhost:9092
MQTT:       mqtt://localhost:1883
```

Stop when done:

```bash
docker compose -f examples/docker-compose.yml down
```

### 3. Upstream Simulators (for Benchmarking & Overhead Measurement)

These simulators provide **known-performance baselines** for measuring VIL's actual processing overhead. By benchmarking the simulator directly (bypass VIL) and then through VIL, you can isolate exactly how much latency and throughput cost VIL adds.

| Simulator | Port | Baseline | Used By | Repository |
|-----------|------|----------|---------|------------|
| **AI Endpoint** (SSE) | 4545 | ~8K req/s | 001, 016-019, 201-206, 301-306, 401-406 | [ai-endpoint-simulator](https://github.com/Vastar-AI/ai-endpoint-simulator) |
| **Credit Data** (NDJSON) | 18081 | ~2K req/s (191K rec/s) | 005, 007-009, 101-107 | [credit-data-simulator](https://github.com/Vastar-AI/credit-data-simulator) |
| **Protocol Sims** | 18090-18092 | — | 017 | business-simulators/protocol-simulators |

**How to measure VIL overhead:**
```
Simulator baseline:  oha -c 200 -n 2000 http://localhost:4545/v1/chat/completions  → X req/s
Via VIL pipeline:    oha -c 200 -n 2000 http://localhost:3080/trigger              → Y req/s
VIL overhead:        (X - Y) / X × 100%
```

**AI Endpoint Simulator** — 5 SSE dialects (OpenAI, Anthropic, Ollama, Cohere, Gemini):

```bash
git clone https://github.com/Vastar-AI/ai-endpoint-simulator.git
cd ai-endpoint-simulator
cargo run --release   # starts on :4545
```

**Credit Data Simulator** — NDJSON/JSON credit records with deterministic seed for reproducible benchmarks:

```bash
git clone https://github.com/Vastar-AI/credit-data-simulator.git
cd credit-data-simulator
cargo build --release
./run_simulator.sh    # starts 4 services on :18081-18084
```

### 4. Load Testing Tool (Optional)

```bash
cargo install oha  # HTTP load testing
```

## Example Tiers

### Tier 1: Basic (001-038)

Foundation examples covering all VIL features. Self-contained where possible, external services clearly documented.

| # | Name | Domain | Requires |
|---|------|--------|----------|
| 001 | AI Gateway | AI Inference Proxy | AI Simulator :4545 |
| 002 | VilApp Gateway | Microservice Gateway | — |
| 003 | Hello Server | Employee Directory | — |
| 004 | REST CRUD | Task Management | — |
| 005 | Multiservice Mesh | Core Banking Ingest | Core Banking :18081 |
| 006 | SHM Extractor | HFT Data Processor | — |
| 007 | Credit NPL Filter | NPL Detection | Core Banking :18081 |
| 008 | Credit Quality | Data Quality Assurance | Core Banking :18081 |
| 009 | Credit Regulatory | OJK SLIK Reporting | Core Banking :18081 |
| 010 | WebSocket Chat | Customer Support Chat | — |
| 011 | GraphQL API | Product Catalog | — |
| 012 | Plugin Database | SaaS Database | **PostgreSQL + Redis** |
| 013 | NATS Worker | Order Processing | **NATS** |
| 014 | Kafka Stream | Transaction Audit | **Kafka** |
| 015 | MQTT IoT | Smart Factory | **MQTT** |
| 016 | AI RAG Gateway | Knowledge Search | AI Simulator :4545 |
| 017 | Production Full | Enterprise Platform | Multiple |
| 018 | AI Multi-Model | Cost Optimizer | AI Simulator :4545 |
| 019 | AI Multi-Model Adv | Multi-Provider | AI Simulator :4545 |
| 020 | A/B Testing | Marketing Campaign | — |
| 021 | WASM FaaS | Business Rules | WASM modules |
| 022 | Sidecar Python | ML Fraud Scoring | — |
| 023 | Hybrid Pipeline | Order Validation | — |
| 024 | LLM Chat | Customer Support Bot | AI Simulator :4545 |
| 025 | RAG Service | Product Knowledge | AI Simulator :4545 |
| 026 | AI Agent | IT Helpdesk | AI Simulator :4545 |
| 027 | VilServer Minimal | Health Check | — |
| 028 | SSE Hub | Live Auction | — |
| 029 | Handler Macros | Developer Playground | — |
| 030 | Tri-Lane Messaging | E-Commerce Orders | — |
| 031 | Mesh Routing | Banking Transaction | — |
| 032 | Failover HA | Payment Gateway | — |
| 033 | SHM Write-Through | Analytics Dashboard | — |
| 034 | Blocking Task | Credit Risk Scoring | — |
| 035 | Service Module | Hospital Appointments | — |
| 036 | SSE Event Builder | Stock Ticker | — |
| 037 | VilModel Derive | Insurance Claims | — |
| 038 | VilApp DSL | Restaurant Orders | — |

### Tier 2: Pipeline (101-107)

Multi-pipeline patterns with ShmToken and Tri-Lane.

| # | Name | Pattern | Requires |
|---|------|---------|----------|
| 101 | 3-Node Transform | ETL Pipeline | Core Banking :18081 |
| 102 | Fan-Out Scatter | Risk Segmentation | Core Banking :18081 |
| 103 | Fan-In Gather | Data Aggregator | Core Banking :18081 |
| 104 | Diamond Topology | Credit Report Views | Core Banking :18081 |
| 105 | Multi-Workflow | Financial Data Hub | Core Banking + AI Sim |
| 106 | SSE Standard | IoT Sensor Dashboard | Core Banking :18081 |
| 107 | Process Traced | Supply Chain Tracking | Core Banking :18081 |

### Tier 3: LLM (201-206)

| # | Name | Unique Pattern | Requires |
|---|------|----------------|----------|
| 201 | Basic Chat | Medical Triage | AI Simulator :4545 |
| 202 | Multi-Model | Translation Pipeline | AI Simulator :4545 |
| 203 | Code Review | Tool Execution | AI Simulator :4545 |
| 204 | Translator | Batch Streaming | AI Simulator :4545 |
| 205 | Summarizer | Chunking Pipeline | AI Simulator :4545 |
| 206 | Decision Routing | Insurance Underwriting | AI Simulator :4545 |

### Tier 4: RAG (301-306)

| # | Name | RAG Pattern | Requires |
|---|------|-------------|----------|
| 301 | Vector Search | Internal Wiki | AI Simulator :4545 |
| 302 | Multi-Source | Legal Compliance | AI Simulator :4545 |
| 303 | Hybrid Search | FAQ + Knowledge | AI Simulator :4545 |
| 304 | Citation | Academic Research | AI Simulator :4545 |
| 305 | Guardrail | Healthcare Safety | AI Simulator :4545 |
| 306 | AI Event | Customer Support | AI Simulator :4545 |

### Tier 5: Agent (401-406)

| # | Name | Tool Pattern | Requires |
|---|------|-------------|----------|
| 401 | Calculator | Financial Calc | AI Simulator :4545 |
| 402 | HTTP Researcher | Market Research | AI Simulator :4545 |
| 403 | Code File | DevOps Incident | AI Simulator :4545 |
| 404 | CSV Analyst | Business Intelligence | AI Simulator :4545 |
| 405 | ReAct Multi-Tool | Autonomous Research | AI Simulator :4545 |
| 406 | Handler SHM | Fraud Detection | AI Simulator :4545 |

## Running Examples

```bash
# Build and run any example
cargo run --release -p vil-basic-hello-server

# Run with environment variable for external services
DATABASE_URL=postgres://postgres:vilpass@localhost:5432/vil_demo \
  cargo run --release -p vil-basic-plugin-database

# Load test
oha -c 200 -n 2000 http://localhost:8080/api/hello/greet/VIL
```

## VIL Way Patterns (All Examples)

Every example uses VIL patterns — zero plain axum:

```rust
// Body: zero-copy from ExchangeHeap
async fn handler(ctx: ServiceCtx, body: ShmSlice) -> VilResponse<T> {
    let input: Request = body.json().expect("JSON");  // SIMD deserialization
    let store = ctx.state::<Arc<Store>>()?;            // Tri-Lane state access
    VilResponse::ok(response)
}
```

## License

Apache-2.0 / MIT
