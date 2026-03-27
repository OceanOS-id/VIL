# 006 — SHM Extractor Demo

VX_APP demonstrating ShmSlice and ShmContext extractors with ExchangeHeap zero-copy body handling, blocking thread pool for CPU-bound work, and SHM region statistics.

| Property | Value |
|----------|-------|
| **Pattern** | VX_APP |
| **Token** | GenericToken (server mode) |
| **Body** | ShmSlice (zero-copy) |
| **Context** | ShmContext (ExchangeHeap stats) |
| **Transform** | N/A |

## Architecture

```
POST /api/shm-demo/ingest   -> ShmSlice (body -> SHM, zero-copy read)
POST /api/shm-demo/compute  -> blocking_with (CPU-bound thread pool)
GET  /api/shm-demo/shm-stats -> ShmContext (region statistics)
GET  /api/shm-demo/benchmark -> minimal throughput endpoint
```

## Key VIL Features Used

- `ShmSlice` extractor: HTTP body -> ExchangeHeap (1 copy), handler reads from SHM (0 copies)
- `ShmContext` extractor: read-only SHM metadata and region stats
- `blocking_with()` for CPU-bound handlers on blocking thread pool
- `VilResponse` typed JSON responses
- Auto-provided `/health`, `/ready`, `/metrics`, `/info` endpoints

## Run

```bash
cargo run -p basic-usage-shm-zerocopy
```

## Test

```bash
curl -X POST http://localhost:8080/api/shm-demo/ingest \
  -H 'Content-Type: application/json' -d '{"sensor":"temp-01","value":23.5}'
curl http://localhost:8080/api/shm-demo/shm-stats
curl http://localhost:8080/api/shm-demo/benchmark
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
| **Throughput** | 40954 req/s |
| **Pattern** | SHM extractor demo |
