# 019 — AI Advanced Multi-Model Router

SDK pipeline with advanced model routing, fallback behavior, confidence scoring, and detailed port naming for production traceability.

| Property | Value |
|----------|-------|
| **Pattern** | SDK_PIPELINE |
| **Token** | GenericToken |
| **Body** | ShmSlice (zero-copy via ExchangeHeap) |
| **Context** | Tri-Lane (Trigger + Data + Control) |
| **Transform** | N/A (passthrough) |

## Architecture

```
POST /route-advanced (:3086)
```

## Key VIL Features Used

- `vil_workflow! with Tri-Lane wiring`
- `#[vil_state], #[vil_event], #[vil_fault] for advanced routing`
- `SseSourceDialect::OpenAi with json_tap`
- `GenericToken for single pipeline`
- `Temperature + max_tokens configuration`

## Run

```bash
cargo run -p basic-usage-ai-multi-model-router-advanced
```

## Test

```bash
curl -N -X POST -H 'Content-Type: application/json' -d '{"prompt": "Compare Rust vs Go"}' http://localhost:3086/route-advanced
```
