# vil_db_dynamodb

VIL Database Plugin for AWS DynamoDB.

Wraps `aws-sdk-dynamodb` with full VIL semantic log integration (`vil_log`).
Every operation emits a `db_log!` entry with timing, hash fields, and op codes.

## Operations

| Method        | op_type | Description                              |
|---------------|---------|------------------------------------------|
| `get_item`    | 0       | Fetch single item by primary key         |
| `put_item`    | 1       | Write/replace item                       |
| `update_item` | 2       | Update item fields via expression        |
| `delete_item` | 3       | Delete item by primary key               |
| `query`       | 4       | Key-condition query with GSI support     |
| `scan`        | 5       | Full-table scan with optional filter     |

## Compliance

- `vil_log` for all logging — no `println!`, `tracing`, or `eprintln!`
- All string fields hashed via `register_str()` → `u32`
- Error type: `DynamoFault` — plain enum, no `String` fields, no `thiserror`
