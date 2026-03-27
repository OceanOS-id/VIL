# VIL Developer Guide — Part 6: CLI, Deployment & Best Practices

**Series:** VIL Developer Guide (6 of 7)
**Previous:** [Part 5 — Infrastructure & Plugins](./005-VIL-Developer_Guide-Infrastructure.md)
**Next:** [Part 7 — Semantic Log System](./007-VIL-Developer_Guide-Semantic-Log.md)
**Last updated:** 2026-03-27

---

## 1. CLI Tools Reference

The `vil` CLI provides project scaffolding, pipeline execution, compilation, and diagnostics:

```bash
vil new <name> --template <template>   # Scaffold a new project
vil run                                 # Run default pipeline
vil run --mock                          # Run with built-in mock server
vil run --file pipeline.vil.yaml      # Run from YAML definition
vil compile --from <lang> --input <file> --output <name>  # Compile DSL to native binary
vil bench -r 2000 -c 200               # Load test a running pipeline
vil registry --processes --ports        # Inspect SHM registry
vil metrics                             # View runtime metrics
vil explain <E-VIL-*>                 # Explain an error code
vil validate <file.yaml>                # Validate a YAML pipeline
vil inspect                             # Inspect project topology
vil build --target vlb                  # Compile to VLB artifact
vil trace --mode live                   # Live request tracing
vil export --output topology.yaml       # Export running topology
```

### 1.1 Available Templates

| Template | Description |
|----------|-------------|
| `stream-filter` | Filter/modify SSE streams (Core Banking SSE default) |
| `ai-inference` | HTTP → SSE proxy for AI inference |
| `webhook-forwarder` | Webhook relay |
| `event-fanout` | One-to-many broadcast |
| `load-balancer` | Multi-backend routing |

### 1.2 `vil compile` — Transpile SDK to Native Binary

The `vil compile` command takes a VIL pipeline written in Python, Go, Java, TypeScript, or YAML and compiles it to a native Rust binary. The compiled binary runs at full Rust-native performance (~3,855 req/s for SSE pipelines) with no FFI overhead.

```bash
vil compile --from <language> --input <source-file> --output <binary-name> [flags]
```

| Flag | Description |
|------|-------------|
| `--from <lang>` | Source language: `python`, `go`, `java`, `typescript`, `yaml` |
| `--input <file>` | Path to the DSL source file |
| `--output <name>` | Name of the output binary |
| `--release` | Build with optimizations (recommended for production) |
| `--target vlb` | Emit VIL Binary format (portable artifact) |
| `--save-manifest` | Save `.vil.yaml` manifest next to the source file |
| `--docker` | Compile inside Docker (no local Rust toolchain needed) |

**Example:**

```bash
# Compile a Python DSL pipeline to a native binary
vil compile --from python --input gateway.py --output gateway --release

# Compile from YAML definition
vil compile --from yaml --input pipeline.vil.yaml --output pipeline --release
```

### 1.3 `vil check` — 9 Validation Checks

| # | Check | Description |
|---|-------|-------------|
| 1 | Schema Validity | YAML structure matches expected schema |
| 2 | DAG Acyclicity | Pipeline graph has no cycles |
| 3 | Node Type Validation | All node types are recognized |
| 4 | Endpoint Uniqueness | No duplicate path+method combinations |
| 5 | Entity Reference | Database entities referenced in handlers exist |
| 6 | Queue Config | Message queue config valid (NATS/Kafka/MQTT) |
| 7 | WASM Module Existence | Referenced .wasm files exist |
| 8 | Sidecar Config | Sidecar definitions have valid protocol |
| 9 | Port Conflicts | No port conflicts between services |

### 1.4 `vil init` — 8 Project Templates

| Template | Description | Generated Sections |
|----------|-------------|-------------------|
| `ai-gateway` | AI gateway with SSE streaming | endpoints + sse_events + providers |
| `rest-crud` | REST API with database | endpoints + database + entities |
| `multi-model-router` | Multi-model AI routing | endpoints + sse_events + routing rules |
| `rag-pipeline` | RAG (Retrieval-Augmented Generation) | endpoints + database + embedding + retrieval |
| `websocket-chat` | WebSocket chat application | endpoints + ws_events + WsHub config |
| `wasm-faas` | WASM FaaS platform | endpoints + vil_wasm + module registry |
| `agent` | AI agent with tool use | endpoints + sse_events + tools + agent config |
| `blank` | Empty template | Minimal vil_version + app metadata |

---

## 2. Transpile-Only SDK (FFI Removed)

VIL uses a **transpile-only** SDK model. The FFI runtime (`vil_ffi`, ctypes/cgo/JNI) has been removed. All cross-language development uses `vil compile` to produce native Rust binaries:

```
┌─────────────────────────────────────────────────────────────┐
│  TRANSPILE MODE (vil compile → native binary)             │
│                                                             │
│  Python/Go/Java/TS  ──vil compile──→  native Rust binary │
│  • Zero FFI overhead, Rust-native performance               │
│  • Single static binary, no runtime dependencies            │
│  • ~3,855 req/s for SSE pipeline (same as hand-written Rust)│
│  • 32 SDK transpile examples (8 per language)               │
└─────────────────────────────────────────────────────────────┘
```

**Workflow:** Write DSL in your language, `vil compile` to native binary for all environments.

### 2.1 Transpile DSL Example (Python)

```python
from vil import VilPipeline, VilServer

pipeline = VilPipeline("credit-gateway")

# Semantic types
pipeline.semantic_type("CreditFilterState", fields={"session_id": "u64", "records_processed": "u64"})
pipeline.semantic_type("NplDetected", kind="event", fields={"session_id": "u64", "record_id": "u32"})

# Error handling (vil_fault)
pipeline.error("CreditFilterFault", variants=["UpstreamTimeout", "InvalidPayload"])

# Server configuration
server = VilServer(pipeline, port=3080)
server.upstream("http://localhost:18081/api/v1/credits/stream")
server.sse(True)

if __name__ == "__main__":
    server.run()
```

**Compile to native binary:**

```bash
vil compile --from python --input gateway.py --output gateway --release
# Produces: ./gateway (native binary, ~3,855 req/s SSE pipeline)
```

### 2.2 SDK Distribution Modes

| Mode | SDK Location | Use Case |
|------|-------------|----------|
| **SDK Mode** (recommended) | `~/.vil/sdk/current/internal/` | Pre-compiled 30+ crates, fastest compile |
| **Source Mode** | Workspace `crates/` directory | For SDK developers |
| **Docker Mode** | Inside Docker container | `vil compile --docker`, no local Rust toolchain |

---

## 3. C/C++ Interoperability via IDL

VIL supports C/C++ interoperability through IR-based header generation. Using `vil_codegen_c`, export Rust interfaces to C headers:

```rust
use vil_codegen_c::generate_header;
let c_header = generate_header(&pipeline_ir);
// Output: vapi_modular_pipeline.h
```

Nodes written in C/C++ can read payloads from the same physical memory (SHM) written by Rust nodes — enabling hybrid Rust/C++/WASM systems.

> **Note:** The FFI runtime (`vil_ffi`) and language-specific bindings (ctypes/cgo/JNI/ffi-napi) have been removed. For cross-language integration, use the **Transpile SDK** (`vil compile`) or the **Sidecar SDK** (UDS + SHM zero-copy IPC).

---

## 4. Universal YAML Compilation System

### 4.1 Pipeline

```
YAML manifest → manifest parse → codegen (Rust source) → cargo build → native binary
```

### 4.2 6 Codegen Modules

| # | Module | YAML Section | Generated Code |
|---|--------|-------------|----------------|
| 1 | Server-Mode | `endpoints:` | `VilApp` + `ServiceProcess` + endpoint routing |
| 2 | Database | `database:` + `entities:` | `MultiPoolManager` + `VilEntity` + CRUD handlers |
| 3 | Message Queue | `message_queue:` | `NatsClient` / `KafkaProducer` / `MqttClient` |
| 4 | WebSocket/SSE | `ws_events:` + `sse_events:` | `WsHub` + `SseHub` + upgrade/subscribe handlers |
| 5 | GraphQL/gRPC | (auto-detected) | `vil_graphql` + `vil_grpc` deps |
| 6 | WASM/Sidecar | `vil_wasm:` + `sidecars:` | `WasmFaaSRegistry` + `SidecarRegistry` |

### 4.3 5+1 Code Execution Modes

| Mode | Backend | Description |
|------|---------|-------------|
| `expr` | Compile-time | Inline expression, embedded in binary |
| `handler` | `vil_server` | Rust handler function, full compile-time check |
| `script (js)` | `vil_script_js` | Sandboxed JavaScript via `boa_engine` |
| `script (lua)` | `vil_script_lua` | Sandboxed Lua via `mlua` |
| `wasm` | `vil_capsule` | WASM pool dispatch via `WasmFaaSRegistry` |
| `sidecar` | `vil_sidecar` | UDS + SHM zero-copy via `SidecarRegistry` |

---

## 5. Health & Metrics Endpoints

Every running VIL pipeline exposes operational endpoints:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Liveness probe — `{"status":"healthy"}` |
| `/ready` | GET | Readiness probe with uptime — `{"status":"ready","uptime_seconds":N}` |
| `/metrics` | GET | Prometheus text exposition format |

### 5.1 Prometheus Metrics

The `/metrics` endpoint exposes:

| Metric | Type | Description |
|--------|------|-------------|
| `vil_requests_total` | counter | Total requests |
| `vil_requests_in_flight` | gauge | Active requests |
| `vil_request_duration_ms` | gauge | Average latency in ms |
| `vil_queue_depth` | gauge | Message queue depth |
| `vil_shm_used_bytes` | gauge | Shared memory usage |
| `vil_route_errors_total` | counter | Route errors |
| `vil_upstream_errors_total` | counter | Upstream errors |

A Grafana dashboard template is provided at `docs/grafana-dashboard.json`.

### 5.2 Admin Endpoints (15 auto-registered)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Kubernetes liveness probe |
| `/ready` | GET | Kubernetes readiness probe |
| `/metrics` | GET | Prometheus metrics |
| `/info` | GET | Server metadata + SHM + handler count |
| `/admin/capsules` | GET | List WASM capsule handlers |
| `/admin/reload/:name` | POST | Hot-reload WASM handler |
| `/admin/diagnostics` | GET | Full runtime diagnostics |
| `/admin/traces` | GET | Recent OpenTelemetry spans |
| `/admin/errors` | GET | Error tracker + patterns |
| `/admin/shm` | GET | SHM region utilization |
| `/admin/config/reload` | POST | Hot config reload |
| `/admin/config/status` | GET | Reload history |
| `/admin/playground` | GET | Embedded API explorer (HTML) |
| `/admin/middleware` | GET | Middleware introspection |
| `/admin/routes` | GET | Registered routes |

---

## 6. Docker Deployment

### 6.1 `.dockerignore`

A `.dockerignore` file is included to optimize Docker build context by excluding `target/`, `.git/`, `business-simulators/`, documentation, and other non-essential directories.

### 6.2 Multi-Stage Build

```dockerfile
# Stage 1: Build
FROM rust:1.76-bookworm AS builder
WORKDIR /build
COPY . .
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim
COPY --from=builder /build/target/release/vil-server /usr/local/bin/
USER 1000
EXPOSE 8080 9090
CMD ["vil-server"]
```

### 6.3 Kubernetes Deployment

VIL includes a K8s Operator CRD (`vil_operator`) for automated deployment:

```yaml
apiVersion: vil.dev/v1alpha1
kind: VilServer
metadata:
  name: my-platform
spec:
  image: ghcr.io/oceanos-id/vil-server:0.1.0
  replicas: 3
  port: 8080
  shm:
    enabled: true
    sizeLimit: "256Mi"
  services:
    - name: auth
      visibility: public
    - name: orders
      visibility: public
  autoscaling:
    enabled: true
    minReplicas: 2
    maxReplicas: 10
    targetCPU: 80
```

---

## 7. Best Practices Summary

1. **Semantic First**: Always use semantic macros (`#[vil_state]`, `#[vil_event]`, `#[vil_fault]`, `#[vil_decision]`) instead of generic `#[message]`.
2. **Lane Separation**: Use Control Lane for session termination signals — never block Data Lane.
3. **No Manual Observability**: Rely on macro annotations (`#[trace_hop]`) for metrics instrumentation.
4. **YAML Topology**: For large distributed systems, consider YAML topology to separate infrastructure configuration from logic code.
5. **Refactor for Humans**: Use *Decomposed Builder* style when your pipeline becomes visually unwieldy.
6. **Zero-Copy Discipline**: Leverage `LoanWrite`/`LoanRead` for high-throughput paths; use `Copy` only for small control messages.
7. **Contract-Driven Design**: Review generated Execution Contracts regularly to ensure topology remains clear and secure.
8. **VASI Compliance**: Use only fixed-size types (`u64`, `u32`, `u16`, `u8`, `bool`) in boundary-crossing structs. No `String` or `Vec`.
9. **Use `vil_new_http`**: The sole HTTP streaming crate — supports SSE (7 dialects) and NDJSON.
10. **Business-Domain Examples**: Start with Core Banking SSE examples (004, 006-008) for fintech use cases, or AI SSE examples (001, 015, 017-018) for LLM/RAG patterns.

---

## 8. Additional Resources

### Core
- [Architecture Overview](./VIL_CONCEPT.md) — layered architecture breakdown
- [SDK Integration Guide](./SDK-Integration-Guide.md) — embedding VIL in applications (Transpile SDK + Sidecar)

### vil-server (Standalone)
- [vil-server Developer Guide](../vil-server/vil-server-guide.md) — full server framework reference (80+ modules)
- [Getting Started with vil-server](../tutorials/tutorial-getting-started-server.md) — step-by-step tutorial
- [Production Deployment Guide](../tutorials/tutorial-production-server.md) — Docker, Kubernetes, monitoring
- [API Reference](../vil-server/API-REFERENCE-SERVER.md) — per-module API documentation (14 crates)

### Community
- [Contributing](./CONTRIBUTING.md) — code style, PR process, guidelines
- [Good First Issues](./GOOD_FIRST_ISSUES.md) — starter tasks for new contributors
- [Changelog](./CHANGELOG.md) — release notes and feature history
- **GitHub**: https://github.com/OceanOS-id/VIL

---

## What's New (2026-03-26)

### 2-Tier Project Split

The VIL ecosystem is now organized as two complementary projects:

```
vil-project/
  vil/              # Core framework (public, open-source)
    crates/         # 101 crates
    examples/       # 49 native examples
    examples-sdk/   # SDK examples (Python/Go/Java/TypeScript)
    sdk/            # Language bindings
    benchmarks/     # Performance benchmarks
  vflow-server/     # VLB provisioning runtime (deployment target)
    src/            # VLB artifact runner
    config/         # Deployment profiles
```

| Repo | Purpose | When to Use |
|------|---------|-------------|
| **vil** | Build pipelines, develop plugins, run 49 examples | Development + CI |
| **vflow-server** | Deploy compiled VLB artifacts to production | Staging + Production |

### 5-Tier Example Structure

Examples are now organized in 5 tiers by complexity:

| Tier | Range | Focus | Count |
|------|-------|-------|-------|
| **Tier 1** | 001-029 | Basic: server, CRUD, mesh, SSE, VilServer, SSE Hub, Macro Demo | 29 |
| **Tier 2** | 101-105 | Multi-pipeline: fan-out, fan-in, diamond, multi-workflow | 5 |
| **Tier 3** | 201-205 | LLM integration (each unique business logic) | 5 |
| **Tier 4** | 301-305 | RAG pipelines (vector, multi-source, hybrid, citation, guardrail) | 5 |
| **Tier 5** | 401-405 | Agent patterns (calculator, HTTP, files, CSV, ReAct loop) | 5 |

**Total: 49 native examples**, plus 32 SDK transpile examples and 9 vflow examples.

### Sync Scripts

Two scripts maintain consistency between the `vil` and `vflow-server` repos:

```bash
# Push shared crates from vil to vflow-server
./scripts/sync-to-vflow.sh

# Pull upstream changes from vflow-server back to vil
./scripts/sync-to-vil.sh
```

Synced artifacts include:
- Shared crate sources (`vil_types`, `vil_shm`, `vil_queue`, `vil_registry`, `vil_rt`)
- VLB format definition and builder
- CLI codegen modules that vflow-server consumes
- Benchmark baselines for regression detection

### CLI Updates

- `vil viz` now supports 6 output formats via the new `vil_viz` crate: `--format html|svg|mermaid|dot|json|ascii`
- `vil build --target vlb` produces artifacts consumable by `vflow-server`
- `vil check` validates both YAML pipelines and Cargo workspace consistency

---

*Previous: [Part 5 — Infrastructure & Plugins](./005-VIL-Developer_Guide-Infrastructure.md)*
*Back to: [Part 1 — Overview & Architecture](./001-VIL-Developer_Guide-Overview.md)*
