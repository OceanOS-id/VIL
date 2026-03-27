# 023 — Hybrid Pipeline (Native + WASM + Sidecar)

Mixed execution model combining Native Rust, WASM FaaS, and Sidecar processes in a single VilApp with failover.

| Property | Value |
|----------|-------|
| **Pattern** | VX_APP |
| **Token** | N/A |
| **Body** | ShmSlice (zero-copy) |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A |

## Architecture

```
POST /validate (Native), POST /price (WASM), POST /fraud (Sidecar), POST /order (Native orchestrator)
```

## Key VIL Features Used

- `Three ExecClass types in one VilApp`
- `WasmFaaSRegistry + SidecarRegistry co-existing`
- `ShmSlice for all POST bodies`
- `VilResponse + VilModel typed output`
- `ServiceProcess with multiple state types`

## Run

```bash
cargo run -p basic-usage-hybrid-pipeline
```

## Test

```bash
curl http://localhost:8080/
curl -X POST http://localhost:8080/order -H 'Content-Type: application/json' -d '{"item":"laptop","qty":1,"amount":999.99}'
```
