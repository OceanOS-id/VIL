# 002 — VilApp Gateway (SseCollect)

VX_APP server that proxies AI inference requests using the built-in SseCollect async client, collecting SSE tokens into a single JSON response.

| Property | Value |
|----------|-------|
| **Pattern** | VX_APP |
| **Token** | N/A (HTTP server) |
| **Body** | ShmSlice (zero-copy) |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A (SseCollect aggregation) |

## Architecture

```
POST /api/trigger (:3081) -> SseCollect -> SSE :4545 -> JSON response
```

## Key VIL Features Used

- `VilApp` + `ServiceProcess` process-oriented architecture
- `ShmSlice` for zero-copy request body
- `SseCollect::post_to()` built-in async SSE client
- `SseDialect::openai()` for OpenAI SSE parsing
- `VilResponse` typed responses
- `VilModel` derive macro
- `.emits::<LlmResponseEvent>()`, `.faults::<LlmFault>()`, `.manages::<LlmUsageState>()`

## Run

```bash
cargo run -p vil-app-gateway --release
```

## Test

```bash
curl -X POST -H "Content-Type: application/json" \
  -d '{"prompt": "Hello"}' http://localhost:3081/api/trigger

oha -m POST -H "Content-Type: application/json" \
  -d '{"prompt": "bench"}' -c 200 -n 2000 http://localhost:3081/api/trigger
```
