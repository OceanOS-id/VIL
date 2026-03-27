# 303 — RAG Hybrid Exact + Semantic

Two-tier search: exact keyword match first (zero latency, no LLM needed), falling back to semantic vector search + LLM only when needed.

| Property | Value |
|----------|-------|
| **Pattern** | VX_APP |
| **Token** | N/A (HTTP server) |
| **Body** | ShmSlice (zero-copy) |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A |

## RAG Pattern

Exact match first (zero latency) -> semantic search fallback -> LLM only if needed -- fundamentally different control flow from always-search RAG

## Architecture

```
POST /api/hybrid (:3112)
  -> Vector search (in-memory cosine similarity)
  -> Context injection into system prompt
  -> SseCollect -> LLM upstream :4545
  -> Post-processing -> VilResponse
```

## Key VIL Features Used

- `Two-tier search: exact match -> semantic fallback`
- `Conditional LLM call (skip if exact hit)`
- `ShmSlice for query body`
- `#[vil_fault] HybridSearchFault`
- `HybridSearchState tracking exact_hits vs semantic_fallbacks`

## Run

```bash
cargo run -p rag-plugin-usage-faq-bot
```

## Test

```bash
curl -X POST -H 'Content-Type: application/json' -d '{"prompt": "How do I reset my password?"}' http://localhost:3112/api/hybrid
```

## System Specs

| Spec | Value |
|------|-------|
| **CPU** | Intel i9-11900F @ 2.50GHz (8C/16T, turbo 5.2GHz) |
| **RAM** | 32GB DDR4 |
| **OS** | Ubuntu 22.04 LTS (kernel 6.8.0) |
| **Rust** | 1.93.1 |
