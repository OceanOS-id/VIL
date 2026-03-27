# VIL Quick Start Guide

**Zero-copy streaming pipelines for Rust**

---

## Installation

### Install VIL CLI

```bash
# Clone the repository
git clone https://github.com/OceanOS-id/VIL.git
cd VIL

# Install the CLI
cargo install --path crates/vil_cli --bin vil
```

Verify installation:
```bash
vil --help
```

---

## Quick Start (30 seconds)

```bash
# Create project
vil new my-gateway --template ai-inference

# Run
cd my-gateway
cargo run
```

---

## Testing the Gateway

### Install oha (HTTP load testing tool)

```bash
# Install oha (HTTP load tester)
cargo install oha

# Or on macOS
brew install oha
```

### Install AI Endpoint Simulator (for full AI inference demo)

```bash
# Clone and run the AI endpoint simulator (supports OpenAI, Anthropic, Ollama, Cohere, Gemini)
git clone https://github.com/Vastar-AI/ai-endpoint-simulator.git
cd ai-endpoint-simulator
cargo run --release
```

The AI endpoint simulator provides multi-dialect SSE endpoints on port 4545.

---

## Usage Examples

### 1. Simple curl test

```bash
curl http://localhost:3080/trigger \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"prompt":"Hello AI!"}'
```

### 2. Load test with oha (3000 requests, 300 concurrent)

```bash
# Basic load test
oha -n 3000 -c 300 http://localhost:3080/trigger

# With POST data
oha -n 3000 -c 300 -m POST \
  -H "Content-Type: application/json" \
  -d '{"prompt":"test"}' \
  http://localhost:3080/trigger
```

Expected output:
```
Summary:
  RPS: 2500.00 (3000 requests in 1.2s)
  Latency:
    min: 10ms
    max: 150ms
    mean: 12ms
    p50: 11ms
    p90: 14ms
    p99: 25ms
```

### 3. Full demo with AI endpoint simulator

```bash
# Terminal 1: Start AI endpoint simulator
git clone https://github.com/Vastar-AI/ai-endpoint-simulator.git
cd ai-endpoint-simulator
cargo run --release

# Terminal 2: Start VIL gateway
cd vil
vil run

# Terminal 3: Test with curl
curl http://localhost:3080/trigger -X POST -d '{"prompt":"What is Rust?"}'

# Terminal 3: Load test
oha -n 3000 -c 300 http://localhost:3080/trigger
```

---

## Available Templates

| Template | Description |
|----------|-------------|
| `ai-inference` | HTTP → SSE proxy for AI inference |
| `webhook-forwarder` | Webhook relay |
| `event-fanout` | One-to-many broadcast |
| `stream-filter` | Filter/modify streams |
| `load-balancer` | Multi-backend routing |

---

## API Layers

### Layer 1: Simple (5 lines)

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    vil_sdk::http_gateway()
        .listen(3080)
        .upstream("http://localhost:4545")
        .run()?;
    Ok(())
}
```

### Layer 2: Customizable (20 lines)

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut pipeline = vil_sdk::Pipeline::new("my-gateway");

    let sink = pipeline.http_sink()
        .port(3080).path("/trigger")
        .build();

    let source = pipeline.http_source()
        .url("http://localhost:4545/v1/chat/completions")
        .format(vil_sdk::http::HttpFormat::SSE)
        .build();

    pipeline.route(sink, source, vil_sdk::RouteMode::LoanWrite);
    pipeline.run()?;
    Ok(())
}
```

---

## CLI Commands

```bash
# Create project
vil new <name> --template <template>

# Run pipeline
vil run

# Run with mock backend
vil run --mock

# Run from YAML file
vil run --file pipeline.vil.yaml

# Run benchmark
vil bench --requests 1000 --concurrency 10

# Initialize in existing project
vil init

# Inspect SHM registry
vil registry --processes --ports --samples

# View metrics
vil metrics
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    VIL Pipeline                          │
├─────────────────┬─────────────────┬─────────────────────────┤
│    DATA LANE   │  CONTROL LANE  │      FAULT LANE        │
├─────────────────┼─────────────────┼─────────────────────────┤
│ Token streams  │ Backpressure   │ Error details           │
│ JSON payloads  │ Flow control   │ Retry signals           │
│ Zero-copy     │ Low latency    │ Graceful degradation    │
└─────────────────┴─────────────────┴─────────────────────────┘
```

---

## Performance

- **Throughput**: 4,000+ req/s on laptop
- **Latency**: < 1ms overhead (P99)
- **Memory**: ~10MB baseline

---

## Tutorials — Pipeline

- [Tutorial Part 1: Hello Pipeline](./tutorials/tutorial-01-hello-pipeline.md)
- [Tutorial Part 2: Custom Nodes](./tutorials/tutorial-02-custom-nodes.md)
- [Tutorial Part 3: Tri-Lane Deep Dive](./tutorials/tutorial-03-trilane.md)
- [Tutorial Part 4: Production Deployment](./tutorials/tutorial-04-production.md)

## vil-server — Microservice Framework

Build microservices with zero-copy SHM, Tri-Lane mesh, and auto-observability:

```bash
vil server new my-api
cd my-api
cargo run
```

```rust
use vil_server::prelude::*;

#[tokio::main]
async fn main() {
    VilServer::new("my-api")
        .port(8080)
        .route("/", get(|| async { "Hello from vil-server!" }))
        .run()
        .await;
}
```

**Learn more:**
- [Getting Started with vil-server](./tutorials/tutorial-getting-started-server.md)
- [vil-server Developer Guide](./vil-server/vil-server-guide.md) (70+ modules)
- [Production Deployment](./tutorials/tutorial-production-server.md)
- [API Reference](./vil-server/API-REFERENCE-SERVER.md)

---

## Support

- GitHub: https://github.com/OceanOS-id/VIL
- Issues: https://github.com/OceanOS-id/VIL/issues
- Discussions: https://github.com/OceanOS-id/VIL/discussions
