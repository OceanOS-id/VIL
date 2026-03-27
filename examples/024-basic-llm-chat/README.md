# 024 — LLM Chat (Basic)

VX_APP LLM chat endpoint using SseCollect with OpenAI dialect and semantic LLM type registration.

| Property | Value |
|----------|-------|
| **Pattern** | VX_APP |
| **Token** | N/A |
| **Body** | ShmSlice (zero-copy) |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A |

## Architecture

```
POST /api/chat (:3090)
```

## Key VIL Features Used

- `SseCollect::post_to() with SseDialect::openai()`
- `ShmSlice for chat request body`
- `.emits::<LlmResponseEvent>(), .faults::<LlmFault>()`
- `VilResponse typed chat response`
- `ServiceProcess + VilApp`

## Run

```bash
cargo run -p basic-usage-llm-chat
```

## Test

```bash
curl -X POST -H 'Content-Type: application/json' -d '{"prompt": "What is Rust?"}' http://localhost:3090/api/chat
```
