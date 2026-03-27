# 001 — AI Gateway Demo (Webhook + SSE Pipeline)

Decomposed-builder SDK pipeline that receives HTTP POST triggers and streams AI inference responses via SSE (OpenAI dialect).

| Property | Value |
|----------|-------|
| **Pattern** | SDK_PIPELINE |
| **Token** | ShmToken (zero-copy) |
| **Body** | ShmSlice (zero-copy via ExchangeHeap) |
| **Context** | Tri-Lane (Trigger + Data + Control) |
| **Transform** | N/A (passthrough SSE stream) |

## Architecture

```
POST /trigger (:3080)
  -> [HttpSink] --trigger(LoanWrite)--> [HttpSource: SSE :4545]
  <- [HttpSink] <--data(LoanWrite)---- [OpenAI SSE stream]
  <- [HttpSink] <--ctrl(Copy)--------- [stream complete]
```

## Key VIL Features Used

- `vil_workflow!` macro for Tri-Lane wiring
- `#[vil_state]` (InferenceState), `#[vil_event]` (InferenceCompleted), `#[vil_fault]` (InferenceFault)
- `HttpSinkBuilder` + `HttpSourceBuilder` decomposed style
- `SseSourceDialect::OpenAi` with `json_tap` extraction
- `ShmToken` for O(1) publish/recv via SHM
- `VastarRuntimeWorld::new_shared()` for ExchangeHeap init

## Run

```bash
cargo run -p basic-usage-ai-gw-demo --release
```

## Test

```bash
# Single request
curl -N -X POST -H "Content-Type: application/json" \
  -d '{"prompt": "test"}' http://localhost:3080/trigger

# Load test
oha -m POST -H "Content-Type: application/json" \
  -d '{"prompt": "bench"}' -c 200 -n 2000 http://localhost:3080/trigger
```

## Benchmark

> **System:** Intel i9-11900F @ 2.50GHz (8C/16T), 32GB RAM, Ubuntu 22.04, Rust 1.93.1
> **Load:** `oha -c 200 -n 2000`
> **Upstream:** ai-endpoint-simulator on :4545

| Metric | Direct :4545 | Via VIL :3080 | Overhead |
|--------|-------------|---------------|----------|
| **Requests/sec** | 4,763 | **4,142** | ~13% |
| **P50 latency** | 41.9ms | 46.6ms | +4.7ms |
| **P99 latency** | 65.5ms | 88.5ms | +23.0ms |
| **Success rate** | 100% | 100% | — |

VIL adds ~5ms P50 overhead for full Tri-Lane SHM pipeline (zero-copy write, process-hop wake, SSE stream ingestion, Control Lane completion). See [PERFORMANCE_REPORT.md](./PERFORMANCE_REPORT.md) for detailed analysis.

### Key Findings
- **13% throughput overhead** — includes SHM LoanWrite, Tri-Lane routing, SSE parsing
- **P50 +4.7ms** — dominated by Tokio task scheduling under 200 concurrent requests
- **P99.9 106ms** — no unbounded tail; Control Lane drains cleanly under max concurrency
- **76 B/response** — gateway aggregates SSE chunks into single JSON (vs 56 KiB raw SSE)
