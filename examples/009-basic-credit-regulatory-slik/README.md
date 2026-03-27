# 009 — Credit Regulatory SLIK Pipeline

SDK pipeline for OJK regulatory credit reporting that streams credit records from Core Banking and maps fields to SLIK (Sistem Layanan Informasi Keuangan) reporting format.

| Property | Value |
|----------|-------|
| **Pattern** | SDK_PIPELINE |
| **Token** | ShmToken (zero-copy) |
| **Body** | ShmSlice (zero-copy via ExchangeHeap) |
| **Context** | Tri-Lane (Trigger + Data + Control) |
| **Transform** | Field mapping to SLIK regulatory schema (v2.1) |

## Architecture

```
POST /regulatory-stream (:3083)
  -> [RegulatorySink] --trigger--> [RegulatorySource: NDJSON :18081]
  <- [RegulatorySink] <--data----- [SLIK-formatted records]
  <- [RegulatorySink] <--ctrl----- [stream complete]
```

## Key VIL Features Used

- `vil_workflow!` with Tri-Lane wiring
- `.transform()` for SLIK field mapping (no_rekening, nik_debitur, plafon, baki_debet, kualitas_kredit)
- `#[vil_state]`, `#[vil_event]`, `#[vil_fault]` semantic types
- `HttpFormat::NDJSON` bulk mode (1000 records)
- `ShmToken` zero-copy transport

## Run

```bash
# Requires: credit-data-simulator (https://github.com/Vastar-AI/credit-data-simulator)
./run_simulator.sh    # starts Core Banking on :18081
cargo run -p basic-usage-credit-regulatory-pipeline
```

## Test

```bash
curl -N -X POST http://localhost:3083/regulatory-stream \
  -H "Content-Type: application/json" -d '{"report_type": "slik-monthly"}'

oha -m POST --no-tui -H "Content-Type: application/json" \
  -d '{"report_type": "bench"}' -c 100 -n 1000 http://localhost:3083/regulatory-stream
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
| **Throughput** | 683 req/s |
| **Pipeline** | NDJSON + SLIK field mapping |
