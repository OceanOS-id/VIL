# VIL Developer Guide

Welcome to the **VIL** development guide. VIL is a process-oriented intermediate language for ultra-low latency distributed systems — combining compile-time semantic validation with a runtime substrate optimized for zero-copy message passing.

**Crates:** 100+ | **Tests:** 1,425+ | **Protocols:** 7
**License:** Apache-2.0
**GitHub:** https://github.com/OceanOS-id/VIL
**Last updated:** 2026-03-24

---

## Guide Index

This guide has been split into 6 parts for easier navigation and richer content:

| # | Document | Scope |
|---|---------|-------|
| 001 | [Overview & Architecture](./001-VIL-Developer_Guide-Overview.md) | Layered architecture (15+ layers), crate taxonomy (100+ crates), quick start examples, project statistics |
| 002 | [Semantic Types & Memory Model](./002-VIL-Developer_Guide-Semantic-Types.md) | Semantic macros (`vil_state`/`event`/`fault`/`decision`), server macros (`VilModel`, `VilError`, `vil_handler`, `VilSseEvent`, `vil_json`), memory classes, transfer modes, session management, ownership tracking, Execution Contract |
| 003 | [Server Framework](./003-VIL-Developer_Guide-Server-Framework.md) | `VilApp`, `ServiceProcess`, `VxMeshConfig`, Tri-Lane mesh, `vil_endpoint`, `vil_app!` DSL, VX architecture (VxKernel, HttpIngress/Egress), database integration (V6 plugins, V7 semantic layer), configuration system |
| 004 | [Pipeline & HTTP Streaming](./004-VIL-Developer_Guide-Pipeline-Streaming.md) | `vil_workflow!` macro (3 styles), `vil_new_http` (SSE + NDJSON source/sink), 7 SSE dialects, `json_tap`, Layer 1/2/3 API, Core Banking SSE examples (004/006/007/008), AI SSE examples, YAML pipeline definitions, `SseCollect` |
| 005 | [Infrastructure & Plugins](./005-VIL-Developer_Guide-Infrastructure.md) | Resilience & fault model, observability (zero-instrumentation), Observer Dashboard, Trust Zones & WASM FaaS, Sidecar SDK (Python/Go), VIL LSP, AI Plugin Infrastructure (51 crates, 4-tier plugin system, 51/51 VIL Way), SHM Token Architecture |
| 006 | [CLI, Deployment & Best Practices](./006-VIL-Developer_Guide-CLI-Deployment.md) | CLI reference (`vil new`/`run`/`compile`/`check`/`init`), Transpile SDK (Python/Go/Java/TypeScript → native binary, FFI removed), C/C++ IDL interop, YAML compilation (6 codegen modules, 5+1 execution modes), health & metrics endpoints, Docker & Kubernetes deployment, best practices |

---

## Quick Start

### Minimal Server

```rust
use vil_server::prelude::*;

#[tokio::main]
async fn main() {
    let service = ServiceProcess::new("hello")
        .endpoint(Method::GET, "/", get(|| async { "Hello from VIL!" }));

    VilApp::new("hello-server")
        .port(8080)
        .service(service)
        .run()
        .await;
}
```

### Minimal Pipeline

```rust
use vil_sdk::http_gateway;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    http_gateway()
        .listen(3080)
        .upstream("http://localhost:18081/api/v1/credits/stream")
        .sse(true)
        .run()?;
    Ok(())
}
```

### CLI Scaffolding

```bash
vil new my-project --template stream-filter
cd my-project
vil run --mock
```

---

## Key Changes (2026-03-24)

- **`vil_http` archived** — all HTTP streaming pipelines now use `vil_new_http` exclusively.
- **Examples 004, 006, 007, 008** updated from AI-centric SSE to **fintech business-domain SSE** using Core Banking Simulator (port 18081).
- **Developer Guide split** into 6 focused documents for richer content and easier navigation.
- **Stats updated**: 100+ crates, 1,425+ tests (previously reported as 48 crates / 410+ tests).

---

## Additional Resources

### Core
- [Architecture Concept](./VIL_CONCEPT.md) — layered architecture breakdown
- [SDK Integration Guide](./SDK-Integration-Guide.md) — embedding VIL in applications (FFI + Transpile SDK)

### vil-server
- [vil-server Developer Guide](../vil-server/vil-server-guide.md) — full server framework reference
- [Getting Started Tutorial](../tutorials/tutorial-getting-started-server.md) — step-by-step tutorial
- [Production Deployment](../tutorials/tutorial-production-server.md) — Docker, Kubernetes, monitoring
- [API Reference](../vil-server/API-REFERENCE-SERVER.md) — per-module API documentation

### Community
- [Contributing](./CONTRIBUTING.md) — code style, PR process, guidelines
- [Good First Issues](./GOOD_FIRST_ISSUES.md) — starter tasks for new contributors
- [Changelog](./CHANGELOG.md) — release notes and feature history
- **GitHub**: https://github.com/OceanOS-id/VIL
