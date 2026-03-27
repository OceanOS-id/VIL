# 005 — Multi-Service Mesh (Core Banking NDJSON)

Two-node SDK pipeline demonstrating Tri-Lane mesh with ShmToken against a fintech NDJSON data source, including real-time credit record enrichment with risk category and LTV ratio.

| Property | Value |
|----------|-------|
| **Pattern** | SDK_PIPELINE |
| **Token** | ShmToken (zero-copy) |
| **Body** | ShmSlice (zero-copy via ExchangeHeap) |
| **Context** | Tri-Lane (Trigger + Data + Control) |
| **Transform** | Risk category + LTV ratio enrichment |

## Architecture

```
POST /ingest (:3084)
  -> [Gateway] --trigger(LoanWrite)--> [CreditIngest: NDJSON :18081]
  <- [Gateway] <--data(LoanWrite)----- [enriched credit records]
  <- [Gateway] <--ctrl(Copy)---------- [stream complete]
```

## Key VIL Features Used

- `vil_workflow!` macro with Tri-Lane routes
- `#[vil_state]`, `#[vil_event]`, `#[vil_fault]` semantic types
- `HttpFormat::NDJSON` for newline-delimited JSON streaming
- `.transform()` closure for per-record enrichment
- `ShmToken` for multi-pipeline zero-copy transport

## Run

```bash
# Requires: credit-data-simulator (https://github.com/Vastar-AI/credit-data-simulator)
./run_simulator.sh    # starts Core Banking on :18081
cargo run -p basic-usage-multiservice-mesh
```

## Test

```bash
curl -N -X POST http://localhost:3084/ingest \
  -H "Content-Type: application/json" \
  -d '{"request":"stream-credits"}'

oha -m POST --no-tui -H "Content-Type: application/json" \
  -d '{"request":"bench"}' -c 50 -n 500 http://localhost:3084/ingest
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
| **Throughput** | 755 req/s |
| **Pipeline** | NDJSON + enrich transform |
