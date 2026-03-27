# Pipeline Tokens

Tokens control how data moves between pipeline nodes. ShmToken for zero-copy, GenericToken for simplicity.

## ShmToken (Zero-Copy)

32-byte fixed-size token referencing data in the ExchangeHeap (shared memory).

```rust
let (_ir, handles) = vil_workflow! {
    name: "HighPerf",
    token: ShmToken,
    instances: [ sink, source ],
    routes: [ sink.out -> source.in (LoanWrite) ]
};
```

### Properties

| Property | Value |
|----------|-------|
| Size | 32 bytes (fixed) |
| Allocation | Zero-alloc on hot path |
| Transport | SHM offset + length + epoch |
| Throughput | ~8.5M msg/s |
| Multi-pipeline | Yes (shared ExchangeHeap) |

### ShmToken Layout (32 bytes)

```
[offset: u64][length: u32][epoch: u32][region_id: u16][flags: u16][checksum: u64]
```

## GenericToken (In-Memory)

Heap-allocated `Bytes` token for simple single-pipeline use.

```rust
let (_ir, handles) = vil_workflow! {
    name: "Simple",
    // token defaults to GenericToken when omitted
    instances: [ sink, source ],
    routes: [ sink.out -> source.in (LoanWrite) ]
};
```

### Properties

| Property | Value |
|----------|-------|
| Size | Variable (heap-allocated) |
| Allocation | One alloc per message |
| Transport | In-memory Bytes |
| Throughput | ~1.2M msg/s |
| Multi-pipeline | No (isolated) |

## Comparison

| Feature | ShmToken | GenericToken |
|---------|----------|--------------|
| Speed | 8.5M msg/s | 1.2M msg/s |
| Memory | Fixed 32B | Variable heap |
| Zero-copy | Yes | No |
| Multi-pipeline sharing | Yes | No |
| Setup complexity | Requires ExchangeHeap | None |
| Best for | Production, high-throughput | Prototyping, simple pipes |

## When to Use Which

**Use ShmToken when:**
- High throughput required (>100K msg/s)
- Multiple pipelines share data
- Large payloads (avoid copying)
- Production deployment

**Use GenericToken when:**
- Prototyping or testing
- Single pipeline with low volume
- No shared memory available
- Simplicity is priority

## TransferMode

Both token types support two transfer modes in route declarations:

| Mode | Description | Use Case |
|------|-------------|----------|
| `LoanWrite` | Zero-copy borrow; reader returns token when done | Default, highest performance |
| `Copy` | Deep copy; each consumer gets independent data | When consumers modify data |

```rust
routes: [
    source.data -> sink_a.in (LoanWrite),  // Zero-copy borrow
    source.data -> sink_b.in (Copy),       // Independent copy
]
```

> Reference: docs/vil/004-VIL-Developer_Guide-Pipeline-Streaming.md
