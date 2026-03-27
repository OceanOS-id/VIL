# VIL — Process-Oriented Intermediate Language

VIL is an **intermediate language** hosted on Rust that makes high-performance, zero-copy distributed systems accessible to teams that are not Rust experts.

VIL sits **above Rust** — developers write intent through semantic macros, and VIL generates all the plumbing: queue wiring, SHM lifecycle, ownership transfer, observability instrumentation.

## What VIL Provides

| Feature | Description |
|---------|-------------|
| **Semantic Type System** | `#[vil_state]`, `#[vil_event]`, `#[vil_fault]`, `#[vil_decision]` |
| **Process Model** | `#[process]`, ports, failure domains, cleanup policies |
| **Zero-Copy Contracts** | VASI validation, `VRef<T>`, `VSlice<T>`, `Loaned<T>` |
| **Workflow DSL** | `vil_workflow!` macro for topology declaration |
| **Transpile SDK** | Write VIL pipelines in Python/Go/Java/TypeScript, compile to native Rust binary via `vil compile` |
| **WebSocket Semantic** | `#[derive(VilWsEvent)]`, `WsHub` topic-based broadcast |
| **Transfer Modes** | LoanWrite, LoanRead, PublishOffset, Copy, ShareRead, ConsumeOnce |
| **Tri-Lane Protocol** | Trigger/Data/Control separation — no head-of-line blocking |
| **Observability** | `#[trace_hop]`, `#[latency_marker]` — zero manual instrumentation |
| **IR & Validation** | 10 compile-time validation passes, JSON/YAML contract export |

## Deployment

VIL code can be written in **Rust** (native) or in **Python, Go, Java, TypeScript** via the Transpile SDK, and compiled to native Rust binaries:

```
Rust Source ─────────── cargo build ──→ vil-server (native binary)

Python/Go/Java/TS ──── vil compile ──→ native binary (same performance)
  (VilPipeline DSL)    --from python     ~3,855 req/s SSE pipeline
                          --release         Single static binary, no FFI overhead
```

The `sdk/` folder contains language bindings for development (FFI mode). For production, use `vil compile` to transpile DSL source into a native binary. See [`examples-sdk/`](../../examples-sdk/) for 76 runnable examples across 4 languages.

## Core Crates

| Crate | Purpose |
|-------|---------|
| `vil_types` | Core types, markers (Vasi, PodLike), identity (ProcessId, PortId) |
| `vil_shm` | Shared memory allocator (ExchangeHeap) |
| `vil_queue` | Zero-copy SPSC/MPMC descriptor queues |
| `vil_registry` | Distributed atomic routing registry |
| `vil_rt` | Runtime kernel (loan, publish, recv, process registration) |
| `vil_ir` | Semantic IR (ProgramIR, ProcessIR, MessageIR, WorkflowIR) |
| `vil_validate` | 10 compile-time validation passes |
| `vil_macros` | Proc-macros (vil_state, vil_workflow!, VilModel, VilError) |
| `vil_codegen_rust` | IR → Rust code generation |
| `vil_codegen_c` | IR → C header export |
| `vil_sdk` | High-level SDK (Layer 1/2/3 API, HttpSink/HttpSource) |
| `vil_json` | High-performance JSON (serde_json default, sonic-rs SIMD optional) |
| `vil_server_macros` | Proc-macros (vil_handler, VilSseEvent, VilWsEvent, vil_endpoint, vil_app) |

## Documentation

- [VIL Developer Guide](./VIL-Developer-Guide.md) — complete reference
- [SDK Integration Guide](./SDK-Integration-Guide.md) — Transpile SDK + FFI SDK for Python/Go/Java/TypeScript
- [VIL Concept](./VIL_CONCEPT.md) — 10 immutable design principles
- [Architecture Overview](../ARCHITECTURE_OVERVIEW.md) — layered architecture
- [Examples Guide](../EXAMPLES.md) — 18 runnable examples
- [CLI Reference](./VIL-Developer-Guide.md#13-cli-tools-reference) — vil compile, run, bench, inspect

## Quick Start

```rust
use vil_sdk::prelude::*;

#[vil_state]
pub struct InferenceResult {
    pub session_id: u64,
    pub tokens: u32,
    pub latency_us: u64,
}

fn main() {
    vil_sdk::http_gateway()
        .listen(3080)
        .upstream("http://localhost:4545/v1/chat/completions")
        .run();
}
```
