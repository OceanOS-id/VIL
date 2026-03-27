# vil_storage_azure

VIL Storage Plugin — Azure Blob Storage.

Wraps `azure_storage_blobs` with VIL semantic logging compliance:
every operation emits `db_log!` with microsecond timing.

## Operations

- `upload_blob(name, body)` — upload bytes as a block blob
- `download_blob(name)` — download blob as `Bytes`
- `delete_blob(name)` — delete blob
- `list_blobs(prefix)` — list blobs by prefix
- `get_properties(name)` — fetch blob properties without body

## Compliance

- `vil_log` only — no `println!`, no `tracing`
- `AzureFault` plain enum — no `thiserror`, no `String` fields
- `register_str()` used for all hash fields in logs and errors
