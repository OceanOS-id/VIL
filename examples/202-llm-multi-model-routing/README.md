# 202 — LLM Multi-Model Routing

SDK pipeline routing to different models (gpt-4 vs gpt-3.5) via Tri-Lane, using vil_llm::pipeline builder helpers.

| Property | Value |
|----------|-------|
| **Pattern** | SDK_PIPELINE |
| **Token** | GenericToken |
| **Body** | ShmSlice (zero-copy) |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A |

## What Makes This Unique

Two pipeline configs for model comparison -- pipeline::chat_sink() + pipeline::chat_source() helpers from vil_llm

## Architecture

```
POST /multi (:3101)
```

## Key VIL Features Used

- `vil_llm::pipeline::chat_sink() + chat_source() builder helpers`
- `vil_workflow! with Tri-Lane routes`
- `#[vil_fault] MultiModelFault`
- `GenericToken for single pipeline`
- `LlmResponseEvent / LlmFault / LlmUsageState semantic types`

## Run

```bash
cargo run -p llm-plugin-usage-multi-model
```

## Test

```bash
curl -N -X POST -H 'Content-Type: application/json' -d '{"prompt": "Explain monads"}' http://localhost:3101/multi
```

## System Specs

| Spec | Value |
|------|-------|
| **CPU** | Intel i9-11900F @ 2.50GHz (8C/16T, turbo 5.2GHz) |
| **RAM** | 32GB DDR4 |
| **OS** | Ubuntu 22.04 LTS (kernel 6.8.0) |
| **Rust** | 1.93.1 |
