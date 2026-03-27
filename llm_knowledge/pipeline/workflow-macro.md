# vil_workflow! Macro

The `vil_workflow!` macro wires pipeline nodes via Tri-Lane routes with zero-copy transport.

## Basic Syntax

```rust
use vil_sdk::prelude::*;
use vil_sdk::http::{HttpSinkBuilder, HttpSourceBuilder, HttpFormat};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sink = HttpSinkBuilder::new()
        .port(3080)
        .path("/trigger")
        .build();

    let source = HttpSourceBuilder::new()
        .url("http://upstream/stream")
        .format(HttpFormat::SSE)
        .build();

    let (_ir, handles) = vil_workflow! {
        name: "MyPipeline",
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

## Fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Pipeline identity string |
| `token` | No | `ShmToken` (32B zero-copy) or `GenericToken` (default) |
| `instances` | Yes | List of node variables |
| `routes` | Yes | Tri-Lane wiring rules |
| `failover` | No | Failover strategy per route |

## Route Syntax

```
source_node.port -> target_node.port (TransferMode)
```

### Ports

| Port | Direction | Description |
|------|-----------|-------------|
| `.out` | Output | Data leaving a sink node |
| `.in` | Input | Data entering a source/processor node |
| `.data` | Output | Streamed data from source node |
| `.ctrl` | Both | Control signals (backpressure, shutdown) |
| `.trigger` | Output | Trigger lane signal |

### TransferMode

| Mode | Copy Cost | Description |
|------|-----------|-------------|
| `LoanWrite` | Zero-copy | SHM borrow, reader returns when done |
| `Copy` | memcpy | Deep copy, independent ownership |

## Failover

```rust
let (_ir, handles) = vil_workflow! {
    name: "Resilient",
    token: ShmToken,
    instances: [ sink, source ],
    routes: [
        sink.out -> source.in (LoanWrite),
        source.data -> sink.in (LoanWrite),
    ],
    failover: {
        source: Restart { max_retries: 3, delay_ms: 500 },
    }
};
```

## Return Values

| Value | Type | Description |
|-------|------|-------------|
| `_ir` | `PipelineIr` | Compiled pipeline IR (inspectable) |
| `handles` | `Vec<JoinHandle>` | Thread handles for each node |

## Multi-Node Example

```rust
let (_ir, handles) = vil_workflow! {
    name: "ThreeNode",
    token: ShmToken,
    instances: [ sink, processor, source ],
    routes: [
        sink.out -> processor.in (LoanWrite),
        processor.out -> source.in (LoanWrite),
        source.data -> sink.in (LoanWrite),
    ]
};
```

> Reference: docs/vil/004-VIL-Developer_Guide-Pipeline-Streaming.md
