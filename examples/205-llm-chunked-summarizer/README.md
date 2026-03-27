# 205 — LLM Chunked Summarizer Pipeline

SDK pipeline with TRANSFORM step that splits long documents into chunks, builds a summarization prompt for all chunks, and sends to LLM.

| Property | Value |
|----------|-------|
| **Pattern** | SDK_PIPELINE |
| **Token** | GenericToken |
| **Body** | ShmSlice (zero-copy) |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A |

## What Makes This Unique

Chunking pipeline -- .transform() splits document at sentence boundaries, builds multi-chunk summarization prompt, LLM produces per-chunk + merged summary

## Architecture

```
POST /summarize (:3104)
```

## Key VIL Features Used

- `vil_workflow! with .transform() for chunk splitting`
- `vil_llm::pipeline::chat_sink() + chat_source() helpers`
- `Sentence-boundary-aware text splitting`
- `GenericToken for single pipeline`
- `#[vil_fault] ChunkerFault (DocumentTooShort, InvalidChunkSize)`

## Run

```bash
cargo run -p llm-plugin-usage-summarizer
```

## Test

```bash
curl -X POST -H 'Content-Type: application/json' -d '{"text": "Long document...", "max_chunk_size": 500}' http://localhost:3104/summarize
```

## System Specs

| Spec | Value |
|------|-------|
| **CPU** | Intel i9-11900F @ 2.50GHz (8C/16T, turbo 5.2GHz) |
| **RAM** | 32GB DDR4 |
| **OS** | Ubuntu 22.04 LTS (kernel 6.8.0) |
| **Rust** | 1.93.1 |
