# 302 — RAG Multi-Source Fan-In

RAG with TWO separate knowledge bases (tech docs + FAQ), searching both independently, cross-ranking by relevance, and combining top hits.

| Property | Value |
|----------|-------|
| **Pattern** | VX_APP |
| **Token** | N/A (HTTP server) |
| **Body** | ShmSlice (zero-copy) |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A |

## RAG Pattern

Two knowledge bases searched independently, results cross-ranked by relevance score, merged into unified context for LLM -- fundamentally different from single-source RAG

## Architecture

```
POST /api/multi-rag (:3111)
  -> Vector search (in-memory cosine similarity)
  -> Context injection into system prompt
  -> SseCollect -> LLM upstream :4545
  -> Post-processing -> VilResponse
```

## Key VIL Features Used

- `Dual knowledge base search (tech docs + FAQ)`
- `Cross-source ranking and merging`
- `ShmSlice for query body`
- `#[vil_fault] MultiSourceFault`
- `RagQueryEvent / RagIngestEvent / RagFault semantic types`

## Run

```bash
cargo run -p rag-plugin-usage-tech-docs
```

## Test

```bash
curl -X POST -H 'Content-Type: application/json' -d '{"prompt": "How does VIL routing work?"}' http://localhost:3111/api/multi-rag
```

## System Specs

| Spec | Value |
|------|-------|
| **CPU** | Intel i9-11900F @ 2.50GHz (8C/16T, turbo 5.2GHz) |
| **RAM** | 32GB DDR4 |
| **OS** | Ubuntu 22.04 LTS (kernel 6.8.0) |
| **Rust** | 1.93.1 |
