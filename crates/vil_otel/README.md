# vil_otel

VIL OpenTelemetry Export — bridge `vil_obs` metrics and `vil_log` events to OTLP.

## Overview

`vil_otel` is a thin bridge layer that connects VIL's built-in observability
(`vil_obs` counters) and structured logging (`vil_log` slots) to OpenTelemetry.
It does not replace either system — it exports their data to any OTLP-compatible
backend (Jaeger, Tempo, Prometheus via OTLP, etc.).

## Components

| Module      | Purpose                                                        |
|-------------|----------------------------------------------------------------|
| `config`    | `OtelConfig` — endpoint, service name, protocol, attributes    |
| `metrics`   | `MetricsBridge` — vil_obs `CounterSnapshot` → OTel counters    |
| `traces`    | `TracesBridge` — `LogSlotHeader` → OTel span attributes        |
| `error`     | `OtelFault` — plain enum, `register_str`-hashed fault codes    |
| `process`   | `create()` — initialise the bridge from `OtelConfig`           |

## Quick Start

```rust,ignore
use vil_otel::{process, config::OtelConfig};
use vil_otel::metrics::CounterSnapshot;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = OtelConfig::new("my-service")
        .with_endpoint("http://localhost:4317")
        .with_export_interval_ms(5_000);

    let bridge = process::create(config).await?;

    // On each metrics tick, call with a snapshot from vil_obs:
    let snap = CounterSnapshot {
        publishes: 42,
        receives:  40,
        drops:     2,
        ..Default::default()
    };
    bridge.metrics().record_snapshot(&snap);

    Ok(())
}
```

## Protocols

| Protocol     | Default Port | Feature            |
|--------------|--------------|--------------------|
| gRPC (tonic) | 4317         | `OtelProtocol::Grpc` |
| HTTP         | 4318         | `OtelProtocol::Http` |

## Compliance

- Uses `vil_log` only — no `println!` or `tracing` calls.
- Fault codes are plain enum variants hashed via `register_str()`.
- `process.rs` exposes the `create()` constructor function.
