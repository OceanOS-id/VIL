# 025 — RAG Service (Basic)

VX_APP RAG endpoint with embedded context documents, citation-aware system prompt, and semantic RagQueryEvent audit.

| Property | Value |
|----------|-------|
| **Pattern** | VX_APP |
| **Token** | N/A |
| **Body** | ShmSlice (zero-copy) |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A |

## Architecture

```
POST /api/rag (:3091)
```

## Key VIL Features Used

- `SseCollect with json_tap extraction`
- `ShmSlice for RAG query body`
- `.emits::<RagQueryEvent>(), .faults::<RagFault>()`
- `Embedded context documents with [DocN] citation`
- `VilResponse typed output`

## Run

```bash
cargo run -p basic-usage-rag-service
```

## Test

```bash
curl -X POST -H 'Content-Type: application/json' -d '{"prompt": "What is Rust ownership?"}' http://localhost:3091/api/rag
```
