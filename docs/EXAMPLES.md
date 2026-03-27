# VIL Examples Guide

This document describes all the example pipelines included in VIL. Each example demonstrates specific features and can be run with `cargo run --example <name> --release`.

---

## Basic Usage Examples (40 total)

The `examples/` directory contains 18 runnable examples. Each example is classified by its **VIL integration depth**: how much of the VIL semantic layer it uses vs plain Rust/Axum patterns.

> **See [VIL_CONCEPT.md](./vil/VIL_CONCEPT.md) for the authoritative definition of "The VIL Way".**

### VIL Integration Legend

| Symbol | Meaning |
|--------|---------|
| **vil_sdk** | Uses VIL pipeline runtime (HttpSink/HttpSource, vil_workflow!, Tri-Lane, GenericToken) |
| **vil_server** | Uses VIL server framework (VilServer, AppState, auto health/metrics/admin) |
| **SHM** | Uses zero-copy SHM extractors/response (ShmSlice, ShmContext, ShmJson, blocking_with) |
| **Semantic** | Uses VIL semantic types (#[vil_state], #[vil_event], #[vil_fault], VilResponse, VilError) |
| **Mesh** | Uses Tri-Lane mesh (ServiceDef, Visibility, TriLaneRouter, ShmDiscovery) |
| **MQ** | Uses VIL message queue adapters (NatsClient, KafkaProducer, MqttClient, *Bridge) |
| **DB** | Uses VIL database layer (SqlxConfig, GraphQLConfig, VilEntity) |

### Pipeline Examples (vil_sdk)

| # | Example | Port | Layer | VIL Features | External Deps |
|---|---------|------|-------|-----------------|---------------|
| 001 | `001-vil-ai-gw-demo` | 3080 | L3 | vil_sdk, SHM, Tri-Lane | serde_json |
| 006 | `006-basic-usage-ai-stream-filter` | 3081 | L3 | vil_sdk, SHM, Tri-Lane | serde_json |
| 007 | `007-basic-usage-ai-content-moderator` | 3082 | L2 | vil_sdk, SHM, Tri-Lane | serde_json |
| 008 | `008-basic-usage-ai-summarizer-pipeline` | 3083 | L1 | vil_sdk (http_gateway) | serde_json |
| 015 | `015-basic-usage-ai-rag-gateway` | 3084 | L3 | vil_sdk, SHM, Tri-Lane | serde_json |
| 017 | `017-basic-usage-ai-multi-model-router` | 3085 | L2 | vil_sdk, SHM, Tri-Lane | serde_json |
| 018 | `018-basic-usage-ai-multi-model-router-advanced` | 3086 | L3 | vil_sdk, SHM, Tri-Lane | serde_json |

All pipeline examples use **100% VIL runtime** — zero-copy SSE via ExchangeHeap, Tri-Lane reactive protocol, `GenericToken` stream tokens, and `vil_workflow!` macro for topology wiring. No external HTTP client library is used; `vil_http::HttpSource` handles upstream SSE natively.

**API Layers:**
- **L1** = `vil_sdk::http_gateway()` — 5-line zero-config gateway
- **L2** = `vil_sdk::Pipeline::new()` — 20-line customizable pipeline
- **L3** = `vil_workflow!` macro — full Decomposed Builder with explicit port wiring

### Server Examples (vil_server)

| # | Example | Port | Architecture | Key Features |
|---|---------|------|----------------|--------------|
| 002 | `002-basic-usage-hello-server` | 8080 | VilApp + ServiceProcess | VilResponse, ShmContext |
| 003 | `003-basic-usage-rest-crud` | 8080 | ServiceProcess + Extension + prefix | VilModel, HandlerResult, VilError |
| 004 | `004-basic-usage-multiservice-mesh` | 8080 | **3 ServiceProcess + VxMeshConfig** | **Tri-Lane (Data+Trigger)**, Visibility::Internal |
| 005 | `005-basic-usage-shm-zerocopy` | 8080 | VilApp + ServiceProcess | ShmSlice, ShmContext, blocking_with |
| 009 | `009-basic-usage-websocket-chat` | 8080 | ServiceProcess + Extension | VilModel, WebSocket |
| 010 | `010-basic-usage-graphql-api` | 8080 | ServiceProcess | VilModel, HandlerResult, GraphQL |
| 011 | `011-basic-usage-plugin-database` | 8080 | ServiceProcess + Extension | VilModel, SqlxConfig |
| 012 | `012-basic-usage-nats-worker` | 8080 | ServiceProcess + Extension + prefix | VilModel, NATS, HandlerResult |
| 013 | `013-basic-usage-kafka-stream` | 8080 | ServiceProcess + Extension + prefix | VilModel, Kafka, HandlerResult |
| 014 | `014-basic-usage-mqtt-iot-gateway` | 8080 | ServiceProcess + Extension + prefix | VilModel, MQTT, HandlerResult |
| 016 | `016-basic-usage-production-fullstack` | 8080 | Multi-ServiceProcess | FullServerConfig, VilResponse |

### VIL Semantic Patterns Used

All server examples now follow "The VIL Way" as defined in [VIL_CONCEPT.md](./vil/VIL_CONCEPT.md):

| Pattern | Usage |
|---------|-------|
| `VilResponse<T>` | Typed response envelope (replaces `Json<Value>`) |
| `HandlerResult<T>` | Error-aware return type (replaces `Result<Json<V>, (StatusCode, Json<V>)>`) |
| `VilError` | RFC 7807 Problem Detail errors (`.not_found()`, `.bad_request()`, etc.) |
| `#[derive(VilModel)]` | Zero-copy model trait on domain types |
| `#[vil_state]` / `#[vil_event]` / `#[vil_fault]` | Semantic type annotations on pipeline types |
| Typed response structs | Named structs with `#[derive(Serialize)]` (replaces `serde_json::json!()`) |

### AI Plugin Usage Examples

| # | Example | Description | Transport | Dialect |
|---|---------|-------------|-----------|---------|
| 023 | `llm-chat` | Basic LLM chat | VilApp+SseCollect | OpenAI |
| 024 | `rag-service` | RAG query | VilApp+SseCollect | OpenAI |
| 025 | `ai-agent` | Tool-calling agent | VilApp+SseCollect | OpenAI |
| 026 | `llm-basic-chat` | LLM plugin demo | VilApp+SseCollect | OpenAI |
| 027 | `llm-multi-model` | Multi-model routing | vil_sdk | OpenAI |
| 028 | `llm-code-assistant` | Code expert | VilApp+SseCollect | Anthropic |
| 029 | `llm-translator` | Translation | VilApp+SseCollect | Ollama |
| 030 | `llm-summarizer` | Summarizer | VilApp+SseCollect | Cohere |
| 031 | `rag-basic-query` | RAG Rust docs | VilApp+SseCollect | OpenAI |
| 032 | `rag-tech-docs` | VIL architecture | VilApp+SseCollect | OpenAI |
| 033 | `rag-faq-bot` | FAQ Q&A | VilApp+SseCollect | Gemini |
| 034 | `rag-legal-search` | Legal clauses | VilApp+SseCollect | OpenAI |
| 035 | `rag-medical-qa` | Medical guidelines | VilApp+SseCollect | OpenAI |
| 036 | `agent-calculator` | Math agent | VilApp+SseCollect | OpenAI |
| 037 | `agent-researcher` | Research agent | VilApp+SseCollect | OpenAI |
| 038 | `agent-code-review` | Code review | VilApp+SseCollect | Custom |
| 039 | `agent-data-analyst` | Data analysis | VilApp+SseCollect | OpenAI |
| 040 | `agent-multi-tool` | Multi-tool agent | VilApp+SseCollect | Custom |

### Running Examples

```bash
# Server examples (run directly)
cargo run -p basic-usage-hello-server
cargo run -p basic-usage-rest-crud

# Pipeline examples (need AI server on localhost:4545)
cargo run -p basic-usage-ai-stream-filter

# Test server examples
curl http://localhost:8080/
curl http://localhost:8080/health
curl http://localhost:8080/metrics

# Test pipeline examples
curl -N -X POST -H "Content-Type: application/json" \
  -d '{"prompt": "test"}' http://localhost:3081/filter
```

---

---

## Quick Reference

| Example | Focus | Complexity | Duration |
|---------|-------|-----------|----------|
| `semantic_types_demo` | All 4 semantic macros | Beginner | < 1s |
| `camera_pipeline` | Realistic pipeline + observability | Intermediate | < 2s |
| `distributed_topo_demo` | Multi-host failover | Advanced | < 3s |
| `lifecycle_dsl_demo` | Session management | Intermediate | < 1s |
| `fault_tolerance_demo` | HA & resilience | Advanced | < 2s |
| `memory_class_demo` | Memory placement optimization | Intermediate | < 1s |
| `trust_zone_demo` | WASM Capsule sandboxing | Advanced | < 2s |
| `execution_contract_demo` | Contract export & inspection | Beginner | < 1s |
| `webhook_pipeline` | HTTP webhook integration | Intermediate | < 3s |
| `vil_v2_full_demo` | All v2 features | Expert | < 5s |

---

## Beginner Examples

### 1. semantic_types_demo

**What it teaches:**
- `#[vil_state]` — Mutable state data
- `#[vil_event]` — Immutable event logs
- `#[vil_fault]` — Structured errors
- `#[vil_decision]` — Routing decisions
- Lane classification

**Run:**
```bash
cargo run --example semantic_types_demo --release
```

**Expected Output:**
```
[State] Processing video frame #42
[Event] Frame processed successfully
[Decision] Route to analyzer
[Fault] Decode error encountered
Semantic validation complete ✓
```

**Key Takeaway:**
Each semantic type tells VIL *what role* the data plays. This enables compile-time validation and automatic optimization.

---

### 2. execution_contract_demo

**What it teaches:**
- Automatic contract generation
- JSON export format
- Pipeline introspection
- Discovery & documentation

**Run:**
```bash
cargo run --example execution_contract_demo --release -- --dump-contract > contract.json
```

**Output (contract.json):**
```json
{
  "pipeline": "ExecutionContractDemo",
  "version": "1.0.0",
  "instances": [
    {
      "name": "decoder",
      "id": 1,
      "process_type": "FrameDecoder",
      "zone": "NativeTrusted"
    }
  ],
  "routes": [
    {
      "from": "decoder.output",
      "to": "analyzer.input",
      "transfer_mode": "LoanWrite",
      "lane": "Data"
    }
  ],
  "latency_profile": {
    "p50_micros": 12,
    "p99_micros": 45,
    "p999_micros": 120
  }
}
```

**Key Takeaway:**
Contracts are automatically generated and exportable. Use them for:
- Dashboard auto-discovery
- Generating C headers
- Documentation
- Integration tests

---

## Intermediate Examples

### 3. camera_pipeline

**What it teaches:**
- Realistic multi-process pipeline
- Observability without manual instrumentation
- Error handling with `#[vil_fault]`
- `#[trace_hop]` for latency tracking

**Run:**
```bash
cargo run --example camera_pipeline --release
```

**Expected Output:**
```
Camera Pipeline Starting...
[1] Capturing frame #1 (1920x1080)
[2] Decoding JPEG data
[3] Analyzing features
[4] Writing to storage
Frame #1 complete in 234 µs
Throughput: 4270 frames/sec
P99 Latency: 456 µs
```

**Pipeline Flow:**
```
Camera Source → JPEG Decoder → Feature Analyzer → Storage Writer
                        ↓ (on error)
                   Error Handler
```

**Code Structure:**
```rust
#[vil_process]
#[trace_hop]
struct CameraSource { /* ... */ }

#[vil_process]
#[trace_hop]
struct JpegDecoder { /* ... */ }

#[vil_process]
#[trace_hop]
struct FeatureAnalyzer { /* ... */ }
```

**Key Takeaway:**
- Latency is automatically tracked per-hop
- No manual metric collection code
- Observable by design

---

### 4. lifecycle_dsl_demo

**What it teaches:**
- Session creation and teardown
- Early-arrival buffering
- Deterministic resource cleanup
- Control Lane signaling

**Run:**
```bash
cargo run --example lifecycle_dsl_demo --release
```

**Expected Output:**
```
Creating session 1...
Session 1 registered
Data arrived: Message(42)
Session 1 completed
Cleanup successful
```

**Key Concepts:**
- Sessions are units of work with clear lifecycle
- Data can arrive before session is fully registered (buffered)
- Teardown via Control Lane (not TCP timeout)
- Resources freed deterministically

---

### 5. memory_class_demo

**What it teaches:**
- `PagedExchange` (default, ultra-fast local SHM)
- `PinnedRemote` (for DMA/RDMA hardware)
- `ControlHeap` (small control messages)
- `LocalScratch` (process-local only)
- Memory class selection for optimization

**Run:**
```bash
cargo run --example memory_class_demo --release
```

**Expected Output:**
```
Paged Exchange (Default):
  Allocation: 128 bytes
  Latency: 0.8 µs
  Throughput: 1.25M msg/sec

Pinned Remote (RDMA):
  Allocation: 128 bytes
  Latency: 0.3 µs (RDMA capable)
  Throughput: 3.33M msg/sec

Control Heap:
  Allocation: 32 bytes
  Latency: 0.2 µs
  Throughput: 5M msg/sec
```

**Key Takeaway:**
Choose memory class based on hardware and use case. Default `PagedExchange` is excellent; `PinnedRemote` unlocks DMA.

---

### 6. webhook_pipeline

**What it teaches:**
- HTTP webhook integration
- External system triggering
- Protocol bridging
- Request-response patterns

**Run:**
```bash
cargo run --example webhook_pipeline --release
```

**Usage:**
```bash
# In another terminal
curl -X POST http://localhost:8080/webhook \
  -H "Content-Type: application/json" \
  -d '{"action": "process", "data": "hello"}'
```

**Expected Output:**
```
Webhook server listening on 0.0.0.0:8080
Received webhook: POST /webhook
Data: {"action": "process", "data": "hello"}
Processing complete
Response sent: {"status": "ok", "processed": true}
```

**Key Takeaway:**
VIL integrates seamlessly with HTTP ecosystems via `vil_new_http`.

---

## Advanced Examples

### 7. distributed_topo_demo

**What it teaches:**
- Multi-host topology definition
- Network routing automation
- Automatic failover
- Host-aware process placement
- Live rerouting

**Run:**
```bash
cargo run --example distributed_topo_demo --release
```

**Expected Output:**
```
Starting multi-host simulation...

Host 1 (edge_node):
  - Process: ingress
  - Status: active

Host 2 (core_node):
  - Process: processor
  - Status: active

Sending message from ingress to processor...
Message routed via network
Processor received: Message(42)
Latency: 145 µs

Simulating host 2 failure...
Host 2 down - failover triggered
Rerouting to backup_node...
```

**Topology YAML:**
```yaml
hosts:
  edge_node: "10.0.0.1:9000"
  core_node: "10.0.0.2:9000"
  backup_node: "10.0.0.3:9000"

instances:
  - name: ingress @ edge_node
  - name: processor @ core_node
  - name: backup @ backup_node

failover:
  - processor => backup (on: HostDown)
```

**Key Takeaway:**
- Topology is declarative (YAML or Rust)
- Networking is automatic
- Failover is built-in
- No manual connection management

---

### 8. fault_tolerance_demo

**What it teaches:**
- Fault propagation via Control Lane
- Retry strategies
- Error recovery patterns
- Session abort semantics
- HA configurations

**Run:**
```bash
cargo run --example fault_tolerance_demo --release
```

**Expected Output:**
```
Scenario 1: Transient Error (Retry)
  Attempt 1: FAIL (timeout)
  Attempt 2: FAIL (timeout)
  Attempt 3: SUCCESS
  Total latency: 450 µs

Scenario 2: Permanent Error (Abort)
  Attempt 1: FAIL (decode error)
  Abort signaled
  Cleanup complete
  Session terminated

Scenario 3: Failover
  Primary down
  Failover to backup triggered
  Data rerouted
  No message loss
```

**Code Pattern:**
```rust
vil_workflow! {
    failover: [
        primary => backup (on: HostDown, strategy: Immediate),
        processor => retry(3) (on: TransferFailed),
    ]
}
```

**Key Takeaway:**
- Faults are first-class entities
- Control Lane keeps error signals responsive
- Failover is declarative
- No message loss during transitions

---

### 9. trust_zone_demo

**What it teaches:**
- WASM Capsule sandboxing
- Capability-limited execution
- Safe third-party code execution
- Isolation guarantees
- Fail-fast on violations

**Run:**
```bash
cargo run --example trust_zone_demo --release
```

**Expected Output:**
```
Native Process (NativeTrusted):
  - Full substrate access
  - Direct memory operations
  - Result: OK

Wasm Capsule (WasmCapsule):
  - Limited host imports
  - Sandboxed memory
  - Attempting forbidden operation...
  - Capability violation detected!
  - Capsule terminated safely
```

**Code Pattern:**
```rust
#[vil_process(zone = WasmCapsule)]
struct UntrustedPlugin {
    // Limited to exported host functions
}
```

**Key Takeaway:**
- Run untrusted code safely in WASM
- No risk to kernel or other processes
- Fail-fast on violations
- Perfect for plugins

---

### 10. vil_v2_full_demo

**What it teaches:**
- All features combined
- Real-world pipeline architecture
- Performance characteristics
- Full feature showcase

**Run:**
```bash
cargo run --example vil_v2_full_demo --release
```

**Expected Output:**
```
VIL v2 Full Feature Showcase
================================

Semantic Types
  [✓] vil_state
  [✓] vil_event
  [✓] vil_fault
  [✓] vil_decision

Transfer Modes
  [✓] LoanWrite (zero-copy)
  [✓] LoanRead (direct access)
  [✓] Copy (control messages)

Topology
  [✓] Workflow DSL
  [✓] Multi-host routing
  [✓] Live rerouting

Fault Model
  [✓] Error propagation
  [✓] Failover
  [✓] Recovery

Trust Zones
  [✓] Native trusted zone
  [✓] WASM capsule
  [✓] Isolation verified

Observability
  [✓] P99 Latency: 234 µs
  [✓] Throughput: 4.27M msg/sec
  [✓] Error rate: 0.01%

Memory Classes
  [✓] PagedExchange
  [✓] PinnedRemote
  [✓] ControlHeap

Execution Contract
  [✓] Generated contract.json
  [✓] Type schemas exported
  [✓] C headers generated

All features verified! ✓
```

---

## Running Multiple Examples

**Run all examples in sequence:**
```bash
for example in semantic_types_demo camera_pipeline distributed_topo_demo \
                lifecycle_dsl_demo fault_tolerance_demo memory_class_demo \
                trust_zone_demo execution_contract_demo webhook_pipeline \
                vil_v2_full_demo; do
  echo "Running: $example"
  cargo run --example "$example" --release
  echo ""
done
```

**Run with timing:**
```bash
time cargo run --example camera_pipeline --release
```

---

## Creating Your Own Example

### Template

1. Create directory:
```bash
mkdir -p examples/my_example/src
```

2. Create `examples/my_example/Cargo.toml`:
```toml
[package]
name = "my_example"
version = "0.1.0"
edition = "2021"

[dependencies]
vil_rt = { path = "../../crates/vil_rt" }
vil_types = { path = "../../crates/vil_types" }
vil_macros = { path = "../../crates/vil_macros" }
tokio = { version = "1", features = ["full"] }
```

3. Create `examples/my_example/src/main.rs`:
```rust
use vil_types::prelude::*;
use vil_macros::*;
use vil_rt::prelude::*;

#[vil_state]
pub struct MyData {
    pub value: u64,
}

#[vil_process]
#[trace_hop]
struct MyProcessor;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let world = VastarRuntimeWorld::new_shared();
    // Your logic here
    Ok(())
}
```

4. Add to top-level `Cargo.toml`:
```toml
[[example]]
name = "my_example"
path = "examples/my_example/src/main.rs"
```

5. Run:
```bash
cargo run --example my_example --release
```

---

## Performance Characteristics

### Latency (P99)

| Example | Median | P99 | P999 |
|---------|--------|-----|------|
| semantic_types | 8 µs | 45 µs | 180 µs |
| camera_pipeline | 120 µs | 456 µs | 1.2 ms |
| distributed_topo | 145 µs | 520 µs | 2.1 ms |
| memory_class | 0.3–5 µs | 2–50 µs | 10–200 µs |

*Note: Measured on Intel Xeon E5-2680 v4, Ubuntu 22.04, single-threaded*

### Throughput

| Example | Ops/sec | Bytes/sec |
|---------|---------|-----------|
| semantic_types | 125K | 16 MB |
| camera_pipeline | 4.27M | 512 MB |
| memory_class | 1.25M–5M | 160–640 MB |

---

## Troubleshooting Examples

### "Example fails with 'SHM allocation failed'"
```bash
# Increase shared memory
sudo mount -o remount,size=4G /dev/shm

# Or run single-threaded
cargo run --example camera_pipeline --release -- --single-threaded
```

### "High latency compared to expected"
- Use `--release` build (not debug)
- Reduce system load
- Increase shared memory size
- Check for CPU throttling

### "Example hangs or doesn't terminate"
- Press Ctrl+C to interrupt
- Check `ulimit -a` for resource limits
- Verify network connectivity (for distributed examples)

---

## Further Learning

1. **Start here**: `semantic_types_demo`
2. **Go realistic**: `camera_pipeline`
3. **Go distributed**: `distributed_topo_demo`
4. **Read the guide**: [VIL-Developer-Guide.md](./vil/VIL-Developer-Guide.md)
5. **Build your own**: Create a new example from the template

---

## Questions?

- Check [QUICK_START.md](./QUICK_START.md)
- Read [VIL-Developer-Guide.md](./vil/VIL-Developer-Guide.md)
- Open an issue on [GitHub](https://github.com/OceanOS-id/VIL/issues)

---

**Last Updated**: 2026-03-21 | **Status**: Stable | **Version**: 6.0.0