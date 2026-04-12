# 001b/vwfd — AI Gateway Benchmark (VWFD version)

Same business as 001b (SSE proxy to upstream LLM), but using VWFD workflow pattern instead of native VilApp code.

| Property | Value |
|----------|-------|
| **Pattern** | VWFD (YAML workflow + vil_vwfd executor) |
| **Equivalent** | 001b (VilApp native Rust) |
| **Port** | 3082 |
| **Upstream** | http://127.0.0.1:4545/v1/chat/completions |

## Architecture

```
POST /api/gw/trigger
  → VWFD Executor
  → Connector (vastar.http POST to upstream LLM)
  → EndTrigger (extract response)
  → JSON response
```

## Run

```bash
cargo run -p vil-vwfd-ai-gw-benchmark
```

## Test

```bash
# Requires upstream LLM on :4545
curl -X POST http://localhost:3082/api/gw/trigger \
  -H 'Content-Type: application/json' \
  -d '{"prompt":"Hello"}'
```

## Benchmark Results

| Metric | VWFD (this) | VilApp (001b) |
|--------|-------------|---------------|
| Execute latency (stub) | 48.7 µs/op | N/A (native) |
| Throughput (stub) | 20,541 ops/sec | ~28,787 req/s |
| Compile | 797 µs | N/A |

VWFD overhead vs native: ~48µs per workflow execution (compile-time cost amortized).
