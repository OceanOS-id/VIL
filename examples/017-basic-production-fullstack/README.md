# 017 — Production Fullstack

Comprehensive reference showing ALL VIL server features: REST, gRPC, NATS, DB, GraphQL, SHM, Tri-Lane, security, and observability.

| Property | Value |
|----------|-------|
| **Pattern** | VX_APP |
| **Token** | N/A |
| **Body** | N/A |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A |

## Architecture

```
GET /api/stack, GET /api/config, GET /api/sprints, GET /api/middleware
```

## Key VIL Features Used

- `FullServerConfig from vil_server_config`
- `Multiple ServiceProcess (public + admin)`
- `VilResponse typed responses`
- `21 middleware layers documented`
- `18 sprints (S1-S18) with module inventory`

## Run

```bash
cargo run -p basic-usage-production-fullstack
```

## Test

```bash
curl http://localhost:8080/api/stack
curl http://localhost:8080/api/sprints
```
