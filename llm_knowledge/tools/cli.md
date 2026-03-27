# VIL CLI

The `vil` CLI manages project creation, compilation, development, and diagnostics.

## Commands

| Command | Description |
|---------|-------------|
| `vil init <name>` | Create new VIL project from template |
| `vil compile <file.vil.yaml>` | Compile YAML pipeline to Rust |
| `vil build` | Build the project (cargo build) |
| `vil run` | Run the compiled binary |
| `vil dev` | Development mode with hot-reload |
| `vil doctor` | Check system dependencies and config |
| `vil inspect <file.vlb>` | Inspect compiled VLB binary |

## vil init

```bash
vil init my-gateway --template ai-inference
```

### Templates

| Template | Description |
|----------|-------------|
| `ai-inference` | SSE gateway to AI provider |
| `event-fanout` | Multi-sink event distribution |
| `load-balancer` | Round-robin load balancer |
| `stream-filter` | NDJSON filter pipeline |
| `webhook-forwarder` | HTTP webhook proxy |

## vil compile

Compiles `.vil.yaml` pipeline definition to Rust source:

```bash
vil compile pipeline.vil.yaml
# Generates: src/generated_pipeline.rs
```

### YAML Pipeline Format

```yaml
name: CreditFilter
token: ShmToken
sink:
  port: 3080
  path: /trigger
source:
  url: http://upstream/credits/ndjson
  format: ndjson
  transform: filter_npl
```

## vil dev

Development mode with file watching and hot-reload:

```bash
vil dev
# Watches src/ for changes
# Auto-recompiles and restarts
# Opens dashboard at http://localhost:3080/_vil/dashboard/
```

## vil doctor

Checks system readiness:

```bash
vil doctor
# [OK] Rust 1.82+
# [OK] Cargo
# [OK] SHM support (/dev/shm writable)
# [OK] WASM target (wasm32-wasi)
# [OK] Python 3.10+ (for sidecar SDK)
# [OK] Go 1.21+ (for sidecar SDK)
```

## vil inspect

Inspect compiled pipeline binary:

```bash
vil inspect pipeline.vlb
# Nodes: 3 (sink, processor, source)
# Routes: 2 (LoanWrite)
# Token: ShmToken (32 bytes)
# Topology: chain
```

## vil build

```bash
vil build                    # Debug build
vil build --release          # Release build
vil build --target wasm32-wasi  # WASM target
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `VIL_LOG_LEVEL` | Log verbosity (trace/debug/info/warn/error) |
| `VIL_CONFIG_PATH` | Path to vil-server.yaml |
| `VIL_SHM_SIZE` | ExchangeHeap size (default: 64MB) |

> Reference: docs/vil/006-VIL-Developer_Guide-CLI-Deployment.md
