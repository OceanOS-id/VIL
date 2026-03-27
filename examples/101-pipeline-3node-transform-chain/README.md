# 101 — 3-Node Transform Chain

Chained multi-step transform pipeline: normalize (uppercase), enrich (risk score), and classify (HIGH/MEDIUM/LOW) credit records from Core Banking NDJSON stream.

| Property | Value |
|----------|-------|
| **Pattern** | SDK_PIPELINE |
| **Token** | GenericToken |
| **Body** | ShmSlice (zero-copy via ExchangeHeap) |
| **Context** | Tri-Lane (Trigger + Data + Control) |
| **Transform** | 3-step chain: uppercase + risk_score + risk_class |

## Architecture

```
POST /transform (:3090)
  -> [TransformGateway] --trigger(LoanWrite)--> [ChainedTransformSource: NDJSON :18081]
     Transform Step 1: Normalize (uppercase nama_lengkap)
     Transform Step 2: Enrich (compute _risk_score = kol*20 + saldo/1M)
     Transform Step 3: Classify (_risk_class: HIGH/MEDIUM/LOW)
  <- [TransformGateway] <--data(LoanWrite)----- [enriched records]
  <- [TransformGateway] <--ctrl(Copy)---------- [stream complete]
```

## Key VIL Features Used

- `vil_workflow!` macro with Tri-Lane routes (LoanWrite + Copy)
- `.transform()` closure with 3 logical steps in one pass
- `#[vil_state]` (TransformChainState), `#[vil_event]` (TransformChainCompleted), `#[vil_fault]` (TransformChainFault)
- `HttpFormat::NDJSON` for streaming credit records
- `GenericToken` for single-pipeline sample-table routing
- `VastarRuntimeWorld::new_shared()` ExchangeHeap initialization
- `HttpSink::from_builder()` + `HttpSource::from_builder()` node instantiation

## Run

```bash
# Requires: credit-data-simulator (https://github.com/Vastar-AI/credit-data-simulator)
./run_simulator.sh    # starts Core Banking on :18081
cargo run -p 101-pipeline-3node-transform-chain
```

## Test

```bash
# Single request
curl -N -X POST http://localhost:3090/transform \
  -H "Content-Type: application/json" \
  -d '{"request":"chain-transforms"}'

# Load test
oha -m POST --no-tui -H "Content-Type: application/json" \
  -d '{"request":"bench"}' -c 50 -n 500 http://localhost:3090/transform
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
| **Throughput** | 1849 req/s |
| **Pipeline** | 3-step chained transform |
| **Token** | GenericToken |
| **Upstream** | Core Banking NDJSON :18081 (100 records/request) |
