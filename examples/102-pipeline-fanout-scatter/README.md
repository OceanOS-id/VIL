# 102 — Fan-Out Scatter

Two parallel pipelines consuming the same NDJSON source with different filters: Pipeline A keeps NPL records (kol>=3), Pipeline B keeps healthy records (kol<3). Both share one ExchangeHeap via ShmToken.

| Property | Value |
|----------|-------|
| **Pattern** | MULTI_PIPELINE |
| **Token** | ShmToken (shared ExchangeHeap) |
| **Body** | ShmSlice (zero-copy via ExchangeHeap) |
| **Context** | Tri-Lane (Trigger + Data + Control) |
| **Transform** | Pipeline A: NPL filter (kol>=3) / Pipeline B: healthy filter (kol<3) |

## Architecture

```
                Core Banking (:18081 NDJSON)
                 /                         \
     Pipeline A (NPL)              Pipeline B (Healthy)
  :3091/npl (kol>=3)           :3092/healthy (kol<3)
  [NplSink] <-> [NplSource]   [HealthySink] <-> [HealthySource]
                 \                         /
               Shared ExchangeHeap (ShmToken)
```

## Key VIL Features Used

- Two `vil_workflow!` instances sharing one `VastarRuntimeWorld`
- `ShmToken` for concurrent multi-pipeline zero-copy sessions
- `.transform()` filter returning `None` to drop non-matching records
- `#[vil_state]`, `#[vil_event]`, `#[vil_fault]` semantic types
- `HttpFormat::NDJSON` on both pipelines
- 4 workers (2 per pipeline) on shared ExchangeHeap

## Run

```bash
# Requires: credit-data-simulator (https://github.com/Vastar-AI/credit-data-simulator)
./run_simulator.sh    # starts Core Banking on :18081
cargo run -p 102-pipeline-fanout-scatter
```

## Test

```bash
# NPL stream (kolektabilitas >= 3)
curl -N -X POST http://localhost:3091/npl \
  -H "Content-Type: application/json" -d '{"request":"npl"}'

# Healthy stream (kolektabilitas < 3)
curl -N -X POST http://localhost:3092/healthy \
  -H "Content-Type: application/json" -d '{"request":"healthy"}'
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
| **Throughput** | 1830 req/s |
| **Pipeline** | Fan-out scatter (2 parallel pipelines) |
| **Token** | ShmToken (shared ExchangeHeap) |
