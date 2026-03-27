# SDK_PIPELINE Pattern

SDK_PIPELINE is the HTTP streaming pattern using vil_workflow!, HttpSinkBuilder, HttpSourceBuilder, and .transform() for inline processing.

## Minimal Pipeline

```rust
use vil_sdk::prelude::*;
use vil_sdk::http::{HttpSinkBuilder, HttpSourceBuilder, HttpFormat, SseSourceDialect};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sink = HttpSinkBuilder::new()
        .port(3080)
        .path("/trigger")
        .build();

    let source = HttpSourceBuilder::new()
        .url("http://localhost:18081/api/v1/credits/stream")
        .format(HttpFormat::SSE)
        .dialect(SseSourceDialect::Standard)
        .build();

    let (_ir, handles) = vil_workflow! {
        name: "CreditGateway",
        token: ShmToken,
        instances: [ sink, source ],
        routes: [
            sink.out -> source.in (LoanWrite),
            source.data -> sink.in (LoanWrite),
        ]
    };

    for h in handles { h.join().unwrap(); }
    Ok(())
}
```

## HttpSinkBuilder

Accepts incoming HTTP requests (webhook trigger):

```rust
let sink = HttpSinkBuilder::new()
    .port(3080)          // Listen port
    .path("/trigger")    // Endpoint path
    .build();
```

## HttpSourceBuilder

Connects to upstream and streams data:

```rust
// SSE source
let source = HttpSourceBuilder::new()
    .url("http://upstream/stream")
    .format(HttpFormat::SSE)
    .dialect(SseSourceDialect::OpenAi)
    .json_tap("choices[0].delta.content")
    .build();

// NDJSON source
let source = HttpSourceBuilder::new()
    .url("http://upstream/data")
    .format(HttpFormat::NDJSON)
    .build();
```

## .transform() for Inline Processing

Filter, map, or enrich records without a separate processor node:

```rust
let source = HttpSourceBuilder::new()
    .url("http://localhost:18081/api/v1/credits/ndjson")
    .format(HttpFormat::NDJSON)
    .transform(|line: &[u8]| -> Option<Vec<u8>> {
        let record: CreditRecord = serde_json::from_slice(line).ok()?;
        if record.kolektabilitas >= 3 {
            Some(serde_json::to_vec(&record).unwrap())
        } else {
            None  // Drop record
        }
    })
    .build();
```

## ShmToken vs GenericToken

```rust
// ShmToken: 32 bytes, zero-alloc, 8.5M msg/s
let (_ir, handles) = vil_workflow! {
    name: "HighThroughput",
    token: ShmToken,
    instances: [ sink, source ],
    routes: [ sink.out -> source.in (LoanWrite) ]
};

// GenericToken: in-memory Bytes, default, 1.2M msg/s
let (_ir, handles) = vil_workflow! {
    name: "Simple",
    // token defaults to GenericToken
    instances: [ sink, source ],
    routes: [ sink.out -> source.in (LoanWrite) ]
};
```

## Three API Layers

| Layer | Lines | Use Case |
|-------|-------|----------|
| Layer 1: `http_gateway()` | ~5 | Simple proxy |
| Layer 2: `Pipeline::new()` | ~20 | Custom topology |
| Layer 3: `vil_workflow!` | Full | Complete control |

> Reference: docs/vil/004-VIL-Developer_Guide-Pipeline-Streaming.md
