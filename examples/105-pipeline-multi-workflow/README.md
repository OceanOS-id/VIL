# 105 — Multi-Workflow Concurrent

Three independent workflows (SSE + NDJSON + REST) running concurrently in a single binary, all sharing one ExchangeHeap via ShmToken. Demonstrates mixed-protocol composition.

| Property | Value |
|----------|-------|
| **Pattern** | MULTI_WORKFLOW |
| **Token** | ShmToken (shared ExchangeHeap) |
| **Body** | ShmSlice (zero-copy via ExchangeHeap) |
| **Context** | Tri-Lane (Trigger + Data + Control) |
| **Transform** | Workflow 2: risk category + LTV / Workflow 3: source tagging |

## Architecture

```
Workflow 1 — AI Gateway (SSE):
  Sink(:3097/ai) -> Source(SSE :4545, OpenAI dialect)

Workflow 2 — Credit Ingest (NDJSON):
  Sink(:3098/credit) -> Source(NDJSON :18081, risk+LTV enrich)

Workflow 3 — Inventory Check (REST):
  Sink(:3099/inventory) -> Source(REST :18092, source tagging)

All 6 workers share ONE ExchangeHeap (ShmToken)
```

## Key VIL Features Used

- Three `vil_workflow!` instances in one binary
- `ShmToken` for 6 concurrent workers on shared ExchangeHeap
- Mixed `HttpFormat`: SSE + NDJSON + Raw (REST)
- `SseSourceDialect::OpenAi` with `json_tap` extraction
- `.transform()` closures for enrichment and tagging
- `#[vil_state]`, `#[vil_event]`, `#[vil_fault]` semantic types

## Run

```bash
# Requires: credit-data-simulator (https://github.com/Vastar-AI/credit-data-simulator)
./run_simulator.sh    # starts Core Banking on :18081
cargo run -p 105-pipeline-multi-workflow
```

## Test

```bash
curl -N -X POST http://localhost:3097/ai \
  -H "Content-Type: application/json" -d '{"prompt":"test"}'

curl -N -X POST http://localhost:3098/credit \
  -H "Content-Type: application/json" -d '{"request":"credits"}'

curl -N -X POST http://localhost:3099/inventory \
  -H "Content-Type: application/json" -d '{"request":"products"}'
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
| **Throughput** | 2049 req/s |
| **Pipeline** | Multi-workflow concurrent (SSE+NDJSON+REST) |
| **Token** | ShmToken (shared ExchangeHeap, 6 workers) |
