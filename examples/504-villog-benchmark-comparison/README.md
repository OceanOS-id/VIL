# 504 — VIL Log: Benchmark Comparison

A simple throughput comparison between `tracing::info!` and `app_log!`.

## Run

```bash
cargo run --release -p example-504-villog-benchmark-comparison
```

Run in `--release` mode for realistic numbers.

## What it measures

| Test | Description |
|------|-------------|
| `tracing::info!` | 1M events into a no-op tracing subscriber |
| `app_log!` | 1M events into NullDrain via the VIL ring |

## Notes

- VIL serializes KV pairs to msgpack on every call — tracing does not.
- NullDrain discards events; I/O cost is excluded from both tests.
- Use this as a directional comparison, not an absolute benchmark.
