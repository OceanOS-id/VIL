# 602-db-mongo-crud

MongoDB CRUD operations with `db_log!` auto-emit.

## What it shows

- `MongoClient::new()` with a local MongoDB URI
- `insert_one`, `find_one`, `update_one`, `delete_one`
- `db_log!` auto-emitted by `vil_db_mongo` on every operation
- `StdoutDrain::resolved()` output format

## Prerequisites

MongoDB:

```bash
docker run -p 27017:27017 mongo:7
```

## Run

```bash
cargo run -p example-602-db-mongo-crud
```

Without MongoDB, the example prints the config and exits gracefully.
