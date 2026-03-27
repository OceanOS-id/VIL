# 020 — AI A/B Testing Gateway

VX_APP implementing weighted traffic splitting (80/20) between model versions with atomic counters for live A/B metrics.

| Property | Value |
|----------|-------|
| **Pattern** | VX_APP |
| **Token** | N/A |
| **Body** | ShmSlice (zero-copy) |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A |

## Architecture

```
POST /api/ab/infer, GET /api/ab/metrics, POST /api/ab/config
```

## Key VIL Features Used

- `ShmSlice for inference request body`
- `ServiceCtx with Arc<AbState> atomic counters`
- `VilResponse + VilModel typed responses`
- `VilError::bad_request for validation`
- `Dynamic traffic split via AtomicU8`

## Run

```bash
cargo run -p basic-usage-ai-ab-testing-gateway
```

## Test

```bash
curl -X POST http://localhost:8080/api/ab/infer -H 'Content-Type: application/json' -d '{"prompt": "Hello AI", "max_tokens": 100}'
curl http://localhost:8080/api/ab/metrics
```
