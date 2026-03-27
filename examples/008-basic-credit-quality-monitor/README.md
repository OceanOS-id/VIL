# 008 — Credit Data Quality Monitor

SDK pipeline that streams credit records and applies real-time data quality validation rules (NIK length, kolektabilitas range, saldo bounds, dirty flag detection).

| Property | Value |
|----------|-------|
| **Pattern** | SDK_PIPELINE |
| **Token** | ShmToken (zero-copy) |
| **Body** | ShmSlice (zero-copy via ExchangeHeap) |
| **Context** | Tri-Lane (Trigger + Data + Control) |
| **Transform** | Quality annotation (_quality_score: PASS/FAIL, _quality_issues[]) |

## Architecture

```
POST /quality-check (:3082)
  -> [QualityMonitorSink] --trigger--> [QualityCreditSource: NDJSON :18081]
  <- [QualityMonitorSink] <--data----- [annotated records with quality score]
  <- [QualityMonitorSink] <--ctrl----- [stream complete]
```

## Key VIL Features Used

- `vil_workflow!` with Tri-Lane wiring
- `.transform()` with multi-rule validation (5 rules)
- `#[vil_state]`, `#[vil_event]`, `#[vil_fault]` semantic types
- `HttpFormat::NDJSON` streaming with `dirty_ratio=0.3`
- `ShmToken` zero-copy transport

## Run

```bash
# Requires: credit-data-simulator (https://github.com/Vastar-AI/credit-data-simulator)
./run_simulator.sh    # starts Core Banking on :18081
cargo run -p basic-usage-credit-quality-monitor
```

## Test

```bash
curl -N -X POST http://localhost:3082/quality-check \
  -H "Content-Type: application/json" -d '{"check": "full-scan"}'

oha -m POST --no-tui -H "Content-Type: application/json" \
  -d '{"check": "bench"}' -c 50 -n 500 http://localhost:3082/quality-check
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
| **Throughput** | 733 req/s |
| **Pipeline** | NDJSON + quality validate |
