# 804-trigger-cdc-postgres

PostgreSQL Change Data Capture (CDC) trigger using logical replication.

## What it shows

- `create_trigger()` building a `CdcTrigger` from `CdcConfig`
- `TriggerSource::start()` streaming row-change events via pgoutput
- `mq_log!` auto-emitted by `vil_trigger_cdc` on every row change
- `StdoutDrain::resolved()` output format

## Prerequisites

PostgreSQL with logical replication enabled:

```bash
docker run -p 5432:5432 \
  -e POSTGRES_PASSWORD=secret \
  -e POSTGRES_DB=vildb \
  postgres:16 \
  postgres -c wal_level=logical \
           -c max_replication_slots=4 \
           -c max_wal_senders=4
```

Then configure the publication and replication slot:

```sql
-- Connect as postgres
CREATE TABLE orders (id SERIAL PRIMARY KEY, amount INT, status TEXT);
CREATE PUBLICATION vil_pub FOR TABLE orders;
SELECT pg_create_logical_replication_slot('vil_cdc_slot', 'pgoutput');
```

## Run

```bash
cargo run -p example-804-trigger-cdc-postgres
```

Without PostgreSQL, the example documents the setup and exits gracefully.

## Generating CDC events

While the example is running, INSERT rows to trigger CDC events:

```sql
INSERT INTO orders (amount, status) VALUES (50000, 'pending');
INSERT INTO orders (amount, status) VALUES (75000, 'confirmed');
UPDATE orders SET status = 'shipped' WHERE id = 1;
```

Each change emits a `mq_log!` entry with:
- `op_type = 1` (consume)
- `source_hash` = FxHash of the slot name
- `payload_bytes` = row change payload size
