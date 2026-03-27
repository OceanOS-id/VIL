# vil_db_cassandra

VIL Database Plugin for Apache Cassandra / ScyllaDB.

Wraps `scylla` with full VIL semantic log integration (`vil_log`).
Every operation emits a `db_log!` entry with timing, hash fields, and op codes.

## Operations

| Method           | op_type | prepared | Description                         |
|------------------|---------|----------|-------------------------------------|
| `execute`        | 0       | 1        | Execute a prepared statement        |
| `query`          | 0       | 0        | Ad-hoc CQL query                    |
| `batch`          | 4       | 0        | Execute a batch of statements       |
| `execute_paged`  | 0       | 1        | Paginated prepared statement        |

Helper: `prepare` — prepare a CQL statement (no db_log, setup-time only).

## Compliance

- `vil_log` for all logging — no `println!`, `tracing`, or `eprintln!`
- All string fields hashed via `register_str()` → `u32`
- Error type: `CassandraFault` — plain enum, no `String` fields, no `thiserror`
