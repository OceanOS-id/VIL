# 103 — Fan-In Gather

Two independent pipelines consuming DIFFERENT upstream sources (NDJSON credits + REST inventory) and sharing one ExchangeHeap via ShmToken. Client gathers results from both.

| Property | Value |
|----------|-------|
| **Pattern** | MULTI_PIPELINE |
| **Token** | ShmToken (shared ExchangeHeap) |
| **Body** | ShmSlice (zero-copy via ExchangeHeap) |
| **Context** | Tri-Lane (Trigger + Data + Control) |
| **Transform** | Source tagging (_source: CORE_BANKING / INVENTORY_SERVICE) |

## Architecture

```
  Pipeline A: Credit Records (NDJSON)
    Sink(:3093/gather) -> Source(NDJSON :18081)

  Pipeline B: Inventory (REST single-shot)
    Sink(:3094/inventory) -> Source(REST :18092)

  Both share ExchangeHeap (ShmToken)
  Client triggers each independently -> gathers results
```

## Key VIL Features Used

- Two `vil_workflow!` with different `HttpFormat` (NDJSON vs Raw)
- `ShmToken` shared across heterogeneous data sources
- `.transform()` tagging records with `_source` origin
- `#[vil_state]`, `#[vil_event]`, `#[vil_fault]` semantic types
- 4 workers sharing one `VastarRuntimeWorld`

## Run

```bash
# Requires: credit-data-simulator (https://github.com/Vastar-AI/credit-data-simulator)
./run_simulator.sh    # starts Core Banking on :18081
cargo run -p 103-pipeline-fanin-gather
```

## Test

```bash
curl -N -X POST http://localhost:3093/gather \
  -H "Content-Type: application/json" -d '{"request":"credits"}'

curl -N -X POST http://localhost:3094/inventory \
  -H "Content-Type: application/json" -d '{"request":"inventory"}'
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
| **Throughput** | 26455 req/s |
| **Pipeline** | Fan-in gather (2 heterogeneous sources) |
| **Token** | ShmToken (shared ExchangeHeap) |
