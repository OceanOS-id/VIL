# vil_db_elastic

VIL Database Plugin — Elasticsearch / OpenSearch.

Wraps the `elasticsearch` crate with VIL semantic logging compliance:
every operation emits `db_log!` with microsecond timing.

## Operations

- `index(index, id, body)` — index (insert/replace) a document
- `search(index, query)` — execute a search query
- `get(index, id)` — retrieve a document by ID
- `delete(index, id)` — delete a document
- `bulk(index, docs)` — bulk index many documents
- `create_index(index, settings)` — create an index with optional settings

## Compliance

- `vil_log` only — no `println!`, no `tracing`
- `ElasticFault` plain enum — no `thiserror`, no `String` fields
- `register_str()` used for all hash fields in logs and errors
