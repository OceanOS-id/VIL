# Hello Pipeline

Complete minimal vil_workflow! pipeline with ShmToken for SSE streaming.

## Full Example

```rust
use vil_sdk::prelude::*;
use vil_sdk::http::{HttpSinkBuilder, HttpSourceBuilder, HttpFormat, SseSourceDialect};

#[vil_state]
pub struct PipelineState {
    pub session_id: u64,
    pub messages_received: u64,
}

#[vil_fault]
pub enum PipelineFault {
    UpstreamTimeout { elapsed_ms: u64 },
    InvalidPayload,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Sink: accepts HTTP POST from clients
    let sink = HttpSinkBuilder::new()
        .port(3080)
        .path("/trigger")
        .build();

    // Source: connects to upstream SSE endpoint
    let source = HttpSourceBuilder::new()
        .url("http://localhost:18081/api/v1/credits/stream")
        .format(HttpFormat::SSE)
        .dialect(SseSourceDialect::Standard)
        .build();

    // Wire the pipeline with ShmToken for zero-copy transport
    let (_ir, handles) = vil_workflow! {
        name: "HelloPipeline",
        token: ShmToken,
        instances: [ sink, source ],
        routes: [
            sink.out -> source.in (LoanWrite),
            source.data -> sink.in (LoanWrite),
        ]
    };

    // Block until pipeline completes
    for h in handles {
        h.join().unwrap();
    }

    Ok(())
}
```

## Data Flow

```
Client --POST--> HttpSink (:3080/trigger)
                     |
                [Trigger Lane]
                     v
                 HttpSource --GET--> Upstream SSE (:18081)
                     |
                [Data Lane - streaming]
                     v
                 HttpSink --SSE chunks--> Client
```

## Key Components

| Component | Purpose |
|-----------|---------|
| `HttpSinkBuilder` | Accepts HTTP requests, returns SSE stream |
| `HttpSourceBuilder` | Connects to upstream, streams data back |
| `vil_workflow!` | Wires nodes via Tri-Lane routes |
| `ShmToken` | 32-byte zero-copy transport token |
| `LoanWrite` | Zero-copy transfer mode (SHM borrow) |

## Layer 1 Shorthand

For simple gateways, use the 5-line Layer 1 API:

```rust
use vil_sdk::http_gateway;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    http_gateway()
        .listen(3080)
        .upstream("http://localhost:18081/api/v1/credits/stream")
        .sse(true)
        .run()?;
    Ok(())
}
```

> Reference: docs/vil/004-VIL-Developer_Guide-Pipeline-Streaming.md
