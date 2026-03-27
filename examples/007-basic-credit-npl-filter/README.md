# 007 — Credit NPL Stream Filter

SDK pipeline that streams credit records from Core Banking Simulator via NDJSON and filters for Non-Performing Loans (kolektabilitas >= 3) per OJK classification.

| Property | Value |
|----------|-------|
| **Pattern** | SDK_PIPELINE |
| **Token** | ShmToken (zero-copy) |
| **Body** | ShmSlice (zero-copy via ExchangeHeap) |
| **Context** | Tri-Lane (Trigger + Data + Control) |
| **Transform** | NPL filter (kolektabilitas >= 3 pass, others dropped) |

## Architecture

```
POST /filter-npl (:3081)
  -> [NplFilterSink] --trigger--> [NplCreditSource: NDJSON :18081]
  <- [NplFilterSink] <--data----- [NPL records only (kol>=3)]
  <- [NplFilterSink] <--ctrl----- [stream complete]
```

## Key VIL Features Used

- `vil_workflow!` with Tri-Lane wiring (LoanWrite + Copy)
- `.transform()` filter: returns `Some(line)` for NPL, `None` to drop healthy
- `#[vil_state]` (NplFilterState), `#[vil_event]` (NplDetected), `#[vil_fault]` (NplFilterFault)
- `HttpFormat::NDJSON` for streaming credit records
- `ShmToken` zero-copy transport

## Run

```bash
# Requires: credit-data-simulator (https://github.com/Vastar-AI/credit-data-simulator)
./run_simulator.sh    # starts Core Banking on :18081
cargo run -p basic-usage-credit-npl-filter
```

## Test

```bash
curl -N -X POST -H "Content-Type: application/json" \
  -d '{"filter": "npl"}' http://localhost:3081/filter-npl

oha -m POST --no-tui -H "Content-Type: application/json" \
  -d '{"filter": "npl"}' -c 50 -n 500 http://localhost:3081/filter-npl
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
| **Throughput** | 927 req/s |
| **Pipeline** | NDJSON + NPL filter (635 of 1000 pass) |
