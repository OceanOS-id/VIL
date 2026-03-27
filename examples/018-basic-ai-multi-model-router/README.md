# 018 — AI Multi-Model Router

VX_APP that routes AI inference to specific models using SseCollect with json_tap extraction, semantic LLM types.

| Property | Value |
|----------|-------|
| **Pattern** | VX_APP |
| **Token** | N/A |
| **Body** | ShmSlice (zero-copy) |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A |

## Architecture

```
POST /api/route (:3085)
```

## Key VIL Features Used

- `SseCollect::post_to() built-in SSE client`
- `json_tap for choices[0].delta.content extraction`
- `.emits::<LlmResponseEvent>(), .faults::<LlmFault>()`
- `ShmSlice for request body`
- `VilResponse typed output`

## Run

```bash
cargo run -p basic-usage-ai-multi-model-router
```

## Test

```bash
curl -X POST -H 'Content-Type: application/json' -d '{"prompt": "Analyze Rust async"}' http://localhost:3085/api/route
```
