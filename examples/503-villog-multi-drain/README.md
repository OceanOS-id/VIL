# 503 — VIL Log: Multi-Drain Fan-Out

Demonstrates `MultiDrain` routing each log batch to both stdout and a file simultaneously.

## Run

```bash
cargo run -p example-503-villog-multi-drain
```

## What it shows

- `MultiDrain::new().add(drain_a).add(drain_b)` builder pattern
- `StdoutDrain` (compact format) for live terminal output
- `FileDrain` (size-based rotation, 10MB per file, keep 5) for persistence
- Every event goes to **both** destinations — no duplication code needed
- `app_log!`, `access_log!`, `mq_log!` covering business, HTTP, and queue events

## Output

Terminal shows compact colored lines. File is written to `./logs/multi.log`.
