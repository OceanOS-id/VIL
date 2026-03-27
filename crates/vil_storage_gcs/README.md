# vil_storage_gcs

VIL Storage Plugin — Google Cloud Storage.

Wraps `google-cloud-storage` with VIL semantic logging compliance:
every operation emits `db_log!` with microsecond timing.

## Operations

- `upload(name, body)` — upload bytes to GCS object
- `download(name)` — download object as `Bytes`
- `delete(name)` — delete object
- `list(prefix)` — list objects by prefix
- `get_metadata(name)` — fetch object metadata without body

## Compliance

- `vil_log` only — no `println!`, no `tracing`
- `GcsFault` plain enum — no `thiserror`, no `String` fields
- `register_str()` used for all hash fields in logs and errors
