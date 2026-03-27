# Multi-Pipeline Pattern

Multiple vil_workflow! pipelines share a common ExchangeHeap for zero-copy data exchange.

## Fan-Out (One Source to N Sinks)

```rust
use vil_sdk::prelude::*;
use vil_sdk::http::{HttpSinkBuilder, HttpSourceBuilder, HttpFormat, SseSourceDialect};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = HttpSourceBuilder::new()
        .url("http://upstream/stream")
        .format(HttpFormat::SSE)
        .dialect(SseSourceDialect::Standard)
        .build();

    let sink_a = HttpSinkBuilder::new().port(3081).path("/a").build();
    let sink_b = HttpSinkBuilder::new().port(3082).path("/b").build();

    let (_ir, handles) = vil_workflow! {
        name: "FanOut",
        token: ShmToken,
        instances: [ source, sink_a, sink_b ],
        routes: [
            source.data -> sink_a.in (LoanWrite),
            source.data -> sink_b.in (LoanWrite),
        ]
    };

    for h in handles { h.join().unwrap(); }
    Ok(())
}
```

## Fan-In (N Sources to One Sink)

```rust
let source_a = HttpSourceBuilder::new()
    .url("http://provider-a/stream").format(HttpFormat::SSE).build();
let source_b = HttpSourceBuilder::new()
    .url("http://provider-b/stream").format(HttpFormat::SSE).build();
let sink = HttpSinkBuilder::new().port(3080).path("/merged").build();

let (_ir, handles) = vil_workflow! {
    name: "FanIn",
    token: ShmToken,
    instances: [ source_a, source_b, sink ],
    routes: [
        source_a.data -> sink.in (LoanWrite),
        source_b.data -> sink.in (LoanWrite),
    ]
};
```

## Diamond (Fan-Out + Process + Fan-In)

```rust
let (_ir, handles) = vil_workflow! {
    name: "Diamond",
    token: ShmToken,
    instances: [ source, processor_a, processor_b, sink ],
    routes: [
        source.data -> processor_a.in (LoanWrite),
        source.data -> processor_b.in (LoanWrite),
        processor_a.out -> sink.in (LoanWrite),
        processor_b.out -> sink.in (LoanWrite),
    ]
};
```

## Independent Pipelines with Shared Heap

```rust
// Pipeline 1
let (_ir1, h1) = vil_workflow! {
    name: "Pipeline1",
    token: ShmToken,
    instances: [ sink1, source1 ],
    routes: [ sink1.out -> source1.in (LoanWrite) ]
};

// Pipeline 2 (shares ExchangeHeap with Pipeline 1)
let (_ir2, h2) = vil_workflow! {
    name: "Pipeline2",
    token: ShmToken,
    instances: [ sink2, source2 ],
    routes: [ sink2.out -> source2.in (LoanWrite) ]
};
```

## Topology Reference

| Pattern | Example | Description |
|---------|---------|-------------|
| Fan-Out | 101 | One source broadcasts to N consumers |
| Fan-In | 102 | N sources merge into one sink |
| Diamond | 103 | Fan-out + parallel process + fan-in |
| Multi-Workflow | 104 | Independent pipelines, shared SHM |
| Conditional | 105 | Dynamic routing by payload content |

> Reference: docs/vil/004-VIL-Developer_Guide-Pipeline-Streaming.md
