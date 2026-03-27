# 301 — RAG Basic Vector Search

RAG with in-memory vector store using cosine similarity to find relevant documents, then LLM generates answer with context.

| Property | Value |
|----------|-------|
| **Pattern** | VX_APP |
| **Token** | N/A (HTTP server) |
| **Body** | ShmSlice (zero-copy) |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A |

## RAG Pattern

Single document collection with cosine similarity scoring -- simplest RAG pattern: embed query -> search vectors -> top-k retrieval -> LLM with context

## Architecture

```
POST /api/rag (:3110)
  -> Vector search (in-memory cosine similarity)
  -> Context injection into system prompt
  -> SseCollect -> LLM upstream :4545
  -> Post-processing -> VilResponse
```

## Key VIL Features Used

- `Cosine similarity vector search (in-memory)`
- `ShmSlice for query body`
- `SseCollect with context-injected system prompt`
- `#[vil_fault] VectorSearchFault`
- `RagQueryEvent / RagFault / RagIndexState semantic types`

## Run

```bash
cargo run -p rag-plugin-usage-basic-query
```

## Test

```bash
curl -X POST -H 'Content-Type: application/json' -d '{"prompt": "What is Rust ownership?"}' http://localhost:3110/api/rag
```

## System Specs

| Spec | Value |
|------|-------|
| **CPU** | Intel i9-11900F @ 2.50GHz (8C/16T, turbo 5.2GHz) |
| **RAM** | 32GB DDR4 |
| **OS** | Ubuntu 22.04 LTS (kernel 6.8.0) |
| **Rust** | 1.93.1 |
