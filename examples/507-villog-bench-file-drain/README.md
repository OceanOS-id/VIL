# 507 — VIL Log: File Drain Benchmark

Measures end-to-end throughput when logging to a file on disk:
- tracing with `RollingFileAppender` (NonBlocking)
- VIL `FileDrain` (rolling, JSON Lines)

## Run

```bash
cargo run -p example-507-villog-bench-file-drain --release
```
