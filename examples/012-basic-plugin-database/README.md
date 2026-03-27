# 012 — Plugin Manager + Database

Demonstrates vil_db_sqlx plugin with multi-pool management patterns, SqlxConfig, and simulated queries.

| Property | Value |
|----------|-------|
| **Pattern** | VX_APP |
| **Token** | N/A |
| **Body** | ShmSlice (zero-copy) |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A |

## Architecture

```
GET /api/plugin-db/plugins, GET /api/plugin-db/config, POST /api/plugin-db/query, GET /api/plugin-db/pool-stats
```

## Key VIL Features Used

- `SqlxConfig for multi-pool database configuration`
- `ShmSlice for query body`
- `VilResponse typed responses`
- `VilModel derive for all types`
- `ServiceProcess + VilApp`

## Run

```bash
cargo run -p basic-usage-plugin-database
```

## Test

```bash
curl http://localhost:8080/api/plugin-db/plugins
curl -X POST http://localhost:8080/api/plugin-db/query -H 'Content-Type: application/json' -d '{"sql":"SELECT * FROM users LIMIT 5"}'
```
