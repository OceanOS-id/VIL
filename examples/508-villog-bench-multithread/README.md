# 508 — VIL Log: Multi-Thread Contention Benchmark

Measures logging throughput under multi-thread contention:
- 1, 2, 4, 8 threads all emitting simultaneously
- Compares tracing (crossbeam MPMC channel) vs VIL (per-thread SPSC ring)

## Run

```bash
cargo run -p example-508-villog-bench-multithread --release
```
