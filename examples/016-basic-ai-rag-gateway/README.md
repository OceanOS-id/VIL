# 016 — AI RAG Gateway Pipeline

SDK pipeline implementing RAG pattern: query enrichment with context documents via system prompt, SSE streaming from AI upstream.

| Property | Value |
|----------|-------|
| **Pattern** | SDK_PIPELINE |
| **Token** | GenericToken |
| **Body** | ShmSlice (zero-copy via ExchangeHeap) |
| **Context** | Tri-Lane (Trigger + Data + Control) |
| **Transform** | N/A (RAG prompt injection) |

## Architecture

```
POST /rag (:3084)
```

## Key VIL Features Used

- `vil_workflow! with Tri-Lane wiring`
- `#[vil_state], #[vil_event], #[vil_fault] semantic types`
- `HttpFormat::SSE + SseSourceDialect::OpenAi`
- `GenericToken for single pipeline`
- `RAG system prompt with document citation instructions`

## Run

```bash
cargo run -p basic-usage-ai-rag-gateway
```

## Test

```bash
curl -N -X POST -H 'Content-Type: application/json' -d '{"prompt": "What is Rust ownership?"}' http://localhost:3084/rag
```
