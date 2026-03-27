# 022 — Sidecar Python ML

Python ML sidecar integration via Unix Domain Socket + SHM zero-copy IPC, with SidecarRegistry and fraud detection demo.

| Property | Value |
|----------|-------|
| **Pattern** | VX_APP |
| **Token** | N/A |
| **Body** | ShmSlice (zero-copy) |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A |

## Architecture

```
POST /api/fraud/check, GET /api/fraud/status
```

## Key VIL Features Used

- `SidecarConfig + SidecarRegistry for external process management`
- `VilApp::sidecar() topology registration`
- `ShmSlice for fraud check body`
- `ServiceCtx with Arc<SidecarRegistry>`
- `VilResponse + VilModel typed output`

## Run

```bash
cargo run -p basic-usage-sidecar-python
```

## Test

```bash
curl http://localhost:8080/api/fraud/status
curl -X POST http://localhost:8080/api/fraud/check -H 'Content-Type: application/json' -d '{"amount": 15000, "merchant_category": "gambling"}'
```
