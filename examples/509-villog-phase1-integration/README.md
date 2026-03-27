# 509 — Phase 1 → Phase 0 Integration Test

Proves that Phase 1 crates (storage + DB) correctly emit `db_log!` events
through Phase 0's striped SPSC ring pipeline.

## Run

```bash
cargo run -p example-509-villog-phase1-integration --release
```

## What it tests

1. **Functional**: Simulated operations from S3, MongoDB, ClickHouse, Elasticsearch, Neo4j, Cassandra all emit `db_log!` events that arrive at the drain
2. **Throughput**: 500K `db_log!` events at 1, 2, and 4 threads — measures ns/event and M events/s
3. **Integration**: Verifies zero ring drops, latency within budget, multi-thread scaling
