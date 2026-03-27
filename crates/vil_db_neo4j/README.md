# vil_db_neo4j

VIL Database Plugin for Neo4j graph database.

Wraps `neo4rs` with full VIL semantic log integration (`vil_log`).
Every operation emits a `db_log!` entry with timing, hash fields, and op codes.

## Operations

| Method             | op_type | Description                              |
|--------------------|---------|------------------------------------------|
| `execute`          | 0       | Execute Cypher query, return RowStream   |
| `run_transaction`  | 4       | Run query in a transaction + commit      |
| `create_node`      | 1       | CREATE node with label and properties    |
| `match_query`      | 0       | MATCH query, collect all rows            |

## Compliance

- `vil_log` for all logging — no `println!`, `tracing`, or `eprintln!`
- All string fields hashed via `register_str()` → `u32`
- Error type: `Neo4jFault` — plain enum, no `String` fields, no `thiserror`
