# 304 — RAG Citation Extraction

RAG with post-processing step that extracts [Doc1], [Doc2] references from LLM output and builds structured citations with title, snippet, and relevance.

| Property | Value |
|----------|-------|
| **Pattern** | VX_APP |
| **Token** | N/A (HTTP server) |
| **Body** | ShmSlice (zero-copy) |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A |

## RAG Pattern

Post-processes LLM response to extract citation references ([DocN]) and build structured citations array with title, snippet, relevance score

## Architecture

```
POST /api/cited-rag (:3113)
  -> Vector search (in-memory cosine similarity)
  -> Context injection into system prompt
  -> SseCollect -> LLM upstream :4545
  -> Post-processing -> VilResponse
```

## Key VIL Features Used

- `Citation regex extraction from LLM output`
- `Structured citations array (title, snippet, relevance)`
- `ShmSlice for query body`
- `#[vil_fault] CitationFault (NoCitationsFound, InvalidCitationFormat)`
- `CitationExtractedEvent semantic audit`

## Run

```bash
cargo run -p rag-plugin-usage-legal-search
```

## Test

```bash
curl -X POST -H 'Content-Type: application/json' -d '{"prompt": "What are the termination conditions?"}' http://localhost:3113/api/cited-rag
```

## System Specs

| Spec | Value |
|------|-------|
| **CPU** | Intel i9-11900F @ 2.50GHz (8C/16T, turbo 5.2GHz) |
| **RAM** | 32GB DDR4 |
| **OS** | Ubuntu 22.04 LTS (kernel 6.8.0) |
| **Rust** | 1.93.1 |
