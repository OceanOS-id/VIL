# 021 — WASM FaaS (Real Execution)

Real WASM execution via wasmtime with pre-compiled module pools for pricing, validation, and transform functions.

| Property | Value |
|----------|-------|
| **Pattern** | VX_APP |
| **Token** | N/A |
| **Body** | ShmSlice (zero-copy) |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A |

## Architecture

```
GET /wasm/modules, POST /wasm/pricing, POST /wasm/validation, POST /wasm/transform
```

## Key VIL Features Used

- `WasmFaaSRegistry with pre-warmed instance pools`
- `ShmSlice for function arguments`
- `ServiceCtx with Arc<WasmFaaSRegistry>`
- `VilResponse typed results`
- `Real wasmtime execution (pricing, validation, transform modules)`

## Run

```bash
cargo run -p basic-usage-wasm-faas
```

## Test

```bash
curl http://localhost:8080/wasm/modules
curl -X POST http://localhost:8080/wasm/pricing -H 'Content-Type: application/json' -d '{"function":"calculate_price","args":[1000, 15]}'
```
