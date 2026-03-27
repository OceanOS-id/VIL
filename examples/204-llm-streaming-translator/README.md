# 204 — LLM Streaming Translator

Batch translation endpoint that processes an array of texts, translating each via individual LLM calls with per-item progress tracking.

| Property | Value |
|----------|-------|
| **Pattern** | VX_APP |
| **Token** | N/A |
| **Body** | ShmSlice (zero-copy) |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A |

## What Makes This Unique

Batch input (array of texts) with per-item sequential LLM translation -- each text gets its own SseCollect call with target language system prompt

## Architecture

```
POST /api/translate/batch (:3103)
```

## Key VIL Features Used

- `Sequential SseCollect calls per text item`
- `ShmSlice for batch request body`
- `#[vil_fault] TranslatorFault (EmptyBatch, UnsupportedLanguage)`
- `VilResponse<BatchTranslateResponse> with per-item status`
- `TranslationCompletedEvent semantic audit`

## Run

```bash
cargo run -p llm-plugin-usage-translator
```

## Test

```bash
curl -X POST -H 'Content-Type: application/json' -d '{"texts": ["Hello", "Goodbye"], "target_lang": "id"}' http://localhost:3103/api/translate/batch
```

## System Specs

| Spec | Value |
|------|-------|
| **CPU** | Intel i9-11900F @ 2.50GHz (8C/16T, turbo 5.2GHz) |
| **RAM** | 32GB DDR4 |
| **OS** | Ubuntu 22.04 LTS (kernel 6.8.0) |
| **Rust** | 1.93.1 |
