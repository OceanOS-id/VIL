# 004 — REST CRUD (Task Management)

Full CRUD REST API with in-memory storage using the VX Process-Oriented architecture with ShmSlice body extraction and ServiceCtx state access.

| Property | Value |
|----------|-------|
| **Pattern** | VX_APP |
| **Token** | N/A (HTTP server) |
| **Body** | ShmSlice (zero-copy) |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A |

## Architecture

```
GET    /api/tasks       -> list all
POST   /api/tasks       -> create (ShmSlice body)
GET    /api/tasks/:id   -> get one
PUT    /api/tasks/:id   -> update (ShmSlice body)
DELETE /api/tasks/:id   -> delete
```

## Key VIL Features Used

- `ShmSlice` for zero-copy JSON body extraction
- `ServiceCtx` with `.state::<Store>()` for shared state access
- `VilResponse::ok()`, `VilResponse::created()` typed responses
- `VilError::bad_request()`, `VilError::not_found()` structured errors
- `#[derive(VilModel)]` for domain types
- `ServiceProcess::new().state(store)` for dependency injection

## Run

```bash
cargo run -p basic-usage-rest-crud
```

## Test

```bash
curl http://localhost:8080/api/tasks
curl -X POST http://localhost:8080/api/tasks \
  -H 'Content-Type: application/json' \
  -d '{"title":"Buy groceries","description":"Milk, eggs, bread"}'
curl http://localhost:8080/api/tasks/1
```

## Benchmark

## System Specs

| Spec | Value |
|------|-------|
| **CPU** | Intel i9-11900F @ 2.50GHz (8C/16T, turbo 5.2GHz) |
| **RAM** | 32GB DDR4 |
| **OS** | Ubuntu 22.04 LTS (kernel 6.8.0) |
| **Rust** | 1.93.1 |

| Metric | Value |
|--------|-------|
| **Throughput** | 10228 req/s |
| **Pattern** | VX_APP CRUD |
