# 505 — VIL Log: Tracing Bridge

Shows `VilTracingLayer` bridging the `tracing` ecosystem into the VIL log ring.

## Run

```bash
cargo run -p example-505-villog-tracing-bridge
```

## What it shows

- `VilTracingLayer::new()` installed as the global tracing subscriber
- Third-party library code calling `tracing::info!/warn!/error!` transparently captured
- VIL native `app_log!` and ecosystem tracing events flowing to the same drain
- Zero changes required to library code

## Use case

Adopt VIL logging incrementally: keep existing `tracing` instrumentation, add
`VilTracingLayer` to capture everything in the VIL ring for unified drain routing.
