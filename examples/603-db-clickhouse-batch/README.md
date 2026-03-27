# 603-db-clickhouse-batch

ClickHouse batch INSERT with `BatchInserter` and `db_log!` auto-emit.

## What it shows

- `ChClient::new()` with local ClickHouse config
- `BatchInserter<T>` — buffered inserter flushing every N rows or timeout
- `db_log!` auto-emitted by `vil_db_clickhouse` on every flush
- `StdoutDrain::resolved()` output format

## Prerequisites

ClickHouse:

```bash
docker run -p 8123:8123 -p 9000:9000 clickhouse/clickhouse-server
```

## Run

```bash
cargo run -p example-603-db-clickhouse-batch
```

Without ClickHouse, the example prints the config and exits gracefully.
