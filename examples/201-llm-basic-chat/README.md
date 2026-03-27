# 201 — LLM Basic Chat

Simplest LLM integration: single POST endpoint with system prompt, SSE collection via SseCollect, and semantic ChatCompletedEvent audit.

| Property | Value |
|----------|-------|
| **Pattern** | VX_APP |
| **Token** | N/A |
| **Body** | ShmSlice (zero-copy) |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A |

## What Makes This Unique

Simplest LLM pattern -- single endpoint, single system prompt, one LLM call

## Architecture

```
POST /api/chat (:3100)
```

## Key VIL Features Used

- `SseCollect::post_to() with SseDialect::openai()`
- `ShmSlice + ServiceCtx extractors`
- `#[vil_fault] ChatFault enum`
- `.emits::<LlmResponseEvent>(), .faults::<LlmFault>(), .manages::<LlmUsageState>()`
- `VilResponse typed responses`

## Run

```bash
cargo run -p llm-plugin-usage-basic-chat
```

## Test

```bash
curl -X POST -H 'Content-Type: application/json' -d '{"prompt": "Hello, world!"}' http://localhost:3100/api/chat
```

## System Specs

| Spec | Value |
|------|-------|
| **CPU** | Intel i9-11900F @ 2.50GHz (8C/16T, turbo 5.2GHz) |
| **RAM** | 32GB DDR4 |
| **OS** | Ubuntu 22.04 LTS (kernel 6.8.0) |
| **Rust** | 1.93.1 |
