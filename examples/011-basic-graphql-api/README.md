# 011 — GraphQL Auto-Generated API

Auto-generated GraphQL schema from entity definitions using VilSchemaBuilder with simulated query execution.

| Property | Value |
|----------|-------|
| **Pattern** | VX_APP |
| **Token** | N/A |
| **Body** | ShmSlice (zero-copy) |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A |

## Architecture

```
GET /api/graphql/schema, GET /api/graphql/entities, POST /api/graphql/query
```

## Key VIL Features Used

- `VilSchemaBuilder for auto-generated GraphQL`
- `ShmSlice for POST body`
- `VilResponse typed responses`
- `VilModel derive for all response types`
- `ServiceProcess + VilApp`

## Run

```bash
cargo run -p basic-usage-graphql-api
```

## Test

```bash
curl http://localhost:8080/api/graphql/schema
curl -X POST http://localhost:8080/api/graphql/query -H 'Content-Type: application/json' -d '{"query":"{ orders { id total } }"}'
```
