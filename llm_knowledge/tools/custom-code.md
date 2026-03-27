# Custom Code — Native, WASM, Sidecar

VIL supports 3 execution modes for custom business logic.

## Modes

| Mode | ExecClass | Overhead | Languages | Hot-Deploy |
|------|-----------|----------|-----------|------------|
| **Native** | `Native` | 0 | Rust | No |
| **WASM** | `WasmFaaS` | ~1-5μs | Rust→.wasm, AssemblyScript | Yes |
| **Sidecar** | `SidecarProcess` | ~12μs | Python, Go, Java, any | Yes |

## YAML Manifest

### Native (Rust handler)

```yaml
endpoints:
  - method: POST
    path: /api/enrich
    handler: enrich_handler
    exec_class: AsyncTask
```

### WASM

```yaml
vil_wasm:
  - name: pricing
    wasm_path: ./wasm-modules/pricing.wasm
    pool_size: 4
    functions:
      - name: calculate_price
    sandbox:
      timeout_ms: 5000
      max_memory_mb: 64
```

### Sidecar

```yaml
sidecars:
  - name: ml-scorer
    command: python3
    script: ./sidecars/ml_scorer.py
    methods: [predict, score_batch]
    auto_restart: true
    max_in_flight: 10
```

## Failover Chain

Native → WASM → Sidecar (degrade performance, preserve availability):

```yaml
failover:
  entries:
    - primary: native-handler
      backup: wasm-fallback
      strategy: instant
    - primary: wasm-fallback
      backup: sidecar-fallback
      condition: WasmExecutionFailed
```

## Non-Rust Developers

| Language | Pipeline Definition | Business Logic |
|----------|--------------------|----|
| Python | Transpile SDK → YAML → binary | Sidecar (UDS) |
| Go | Transpile SDK → YAML → binary | Sidecar (UDS) |
| TypeScript | Transpile SDK → YAML → binary | WASM (AssemblyScript) or Sidecar |

> Full guide: docs/vil/CUSTOM_CODE_GUIDE.md
