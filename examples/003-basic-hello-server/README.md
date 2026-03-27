# 003 — Hello Server

Minimal VX_APP demonstrating VIL Way handler patterns: zero-copy body via ShmSlice, Tri-Lane context via ServiceCtx, and SIMD JSON deserialization.

| Property | Value |
|----------|-------|
| **Pattern** | VX_APP |
| **Token** | N/A (HTTP server) |
| **Body** | ShmSlice (zero-copy) |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A |

## Architecture

```
GET  /api/hello/          -> plain text
GET  /api/hello/greet/:n  -> VilResponse<GreetResponse>
POST /api/hello/echo      -> ShmSlice -> VilResponse<EchoResponse>
GET  /api/hello/shm-info  -> ShmContext + ServiceCtx -> VilResponse
```

## Key VIL Features Used

- `ShmSlice` extractor for zero-copy request body from ExchangeHeap
- `ServiceCtx` auto-extracted Tri-Lane context
- `ShmContext` for ExchangeHeap region statistics
- `VilResponse` typed JSON responses
- `ServiceProcess::new()` + `VilApp::new()` process-oriented app

## Run

```bash
cargo run -p vil-basic-hello-server
```

## Test

```bash
curl http://localhost:8080/api/hello/greet/World
curl -X POST http://localhost:8080/api/hello/echo \
  -H 'Content-Type: application/json' -d '{"msg":"hi"}'
curl http://localhost:8080/api/hello/shm-info
curl http://localhost:8080/health
```

## Benchmark

## System Specs

| Spec | Value |
|------|-------|
| **CPU** | Intel i9-11900F @ 2.50GHz (8C/16T, turbo 5.2GHz) |
| **RAM** | 32GB DDR4 |
| **OS** | Ubuntu 22.04 LTS (kernel 6.8.0) |
| **Rust** | 1.93.1 |

| Metric | Value |
|--------|-------|
| **Throughput** | 28787 req/s |
| **Pattern** | VX_APP hello server |
