# 305 — RAG Guardrail Pipeline

RAG with multi-stage guardrail post-processing: PII detection (email/phone/NIK regex), hallucination marker check, redaction, and confidence scoring.

| Property | Value |
|----------|-------|
| **Pattern** | VX_APP |
| **Token** | N/A (HTTP server) |
| **Body** | ShmSlice (zero-copy) |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A |

## RAG Pattern

Multi-stage guardrail pipeline after LLM: PII detection -> hallucination check -> redaction -> guardrail_status (PASS/REDACTED/BLOCKED) + confidence_score

## Architecture

```
POST /api/safe-rag (:3114)
  -> Vector search (in-memory cosine similarity)
  -> Context injection into system prompt
  -> SseCollect -> LLM upstream :4545
  -> Post-processing -> VilResponse
```

## Key VIL Features Used

- `PII detection (email, phone, NIK regex patterns)`
- `Hallucination marker detection`
- `Content redaction pipeline`
- `#[vil_fault] GuardrailFault`
- `GuardrailState tracking pass/redacted/blocked counts`

## Run

```bash
cargo run -p rag-plugin-usage-medical-qa
```

## Test

```bash
curl -X POST -H 'Content-Type: application/json' -d '{"prompt": "What are the symptoms of diabetes?"}' http://localhost:3114/api/safe-rag
```

## System Specs

| Spec | Value |
|------|-------|
| **CPU** | Intel i9-11900F @ 2.50GHz (8C/16T, turbo 5.2GHz) |
| **RAM** | 32GB DDR4 |
| **OS** | Ubuntu 22.04 LTS (kernel 6.8.0) |
| **Rust** | 1.93.1 |
