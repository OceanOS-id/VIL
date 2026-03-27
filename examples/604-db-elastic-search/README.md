# 604-db-elastic-search

Elasticsearch index + search with `db_log!` auto-emit.

## What it shows

- `ElasticClient::new()` with a local Elasticsearch config
- `index()` to insert a document
- `search()` with a match query
- `db_log!` auto-emitted by `vil_db_elastic` on every operation
- `StdoutDrain::resolved()` output format

## Prerequisites

Elasticsearch:

```bash
docker run -p 9200:9200 \
  -e discovery.type=single-node \
  -e xpack.security.enabled=false \
  elasticsearch:8.13.0
```

## Run

```bash
cargo run -p example-604-db-elastic-search
```

Without Elasticsearch, the example prints the config and exits gracefully.
