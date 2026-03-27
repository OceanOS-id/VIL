# 104 — Diamond Topology

Two parallel pipelines consuming the SAME NDJSON source with DIFFERENT transforms: Pipeline A produces NPL summary view, Pipeline B produces full enrichment detail view.

| Property | Value |
|----------|-------|
| **Pattern** | MULTI_PIPELINE |
| **Token** | ShmToken (shared ExchangeHeap) |
| **Body** | ShmSlice (zero-copy via ExchangeHeap) |
| **Context** | Tri-Lane (Trigger + Data + Control) |
| **Transform** | A: NPL summary (compact fields) / B: Full enrichment (risk_score, ltv, aging) |

## Architecture

```
              Core Banking (:18081)
               /                \
     Pipeline A              Pipeline B
   (NPL Summary)         (Full Enrichment)
  :3095/diamond        :3096/diamond-detail
   compact fields       risk_score, ltv_ratio,
   kol>=3 only          risk_class, aging_bucket
               \                /
                Client (gather)
```

## Key VIL Features Used

- Two `vil_workflow!` with divergent `.transform()` logic on same source
- `ShmToken` shared ExchangeHeap for diamond branches
- Summary view: filter + compact projection
- Detail view: full enrichment (risk_score, ltv_ratio, risk_class, aging_bucket)
- `#[vil_state]`, `#[vil_event]`, `#[vil_fault]` semantic types
- `HttpFormat::NDJSON` on both branches

## Run

```bash
# Requires: credit-data-simulator (https://github.com/Vastar-AI/credit-data-simulator)
./run_simulator.sh    # starts Core Banking on :18081
cargo run -p 104-pipeline-diamond-topology
```

## Test

```bash
curl -N -X POST http://localhost:3095/diamond \
  -H "Content-Type: application/json" -d '{"request":"summary"}'

curl -N -X POST http://localhost:3096/diamond-detail \
  -H "Content-Type: application/json" -d '{"request":"detail"}'
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
| **Throughput** | 17685 req/s |
| **Pipeline** | Diamond topology (2 views, same source) |
| **Token** | ShmToken (shared ExchangeHeap) |
