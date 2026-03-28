# Storage Connectors

VIL provides native storage connectors for S3, GCS, and Azure Blob with automatic `#[connector_fault/event/state]` instrumentation.

## Quick Reference

| Connector | Crate | Backends |
|-----------|-------|----------|
| S3 | vil_conn_s3 | AWS S3, MinIO, Cloudflare R2 |
| GCS | vil_conn_gcs | Google Cloud Storage |
| Azure Blob | vil_conn_azure_blob | Azure Blob Storage |

## S3 (vil_conn_s3)

```rust
use vil_conn_s3::{S3Connector, S3Config};

let s3 = S3Connector::new(S3Config {
    bucket: "my-bucket".into(),
    region: "us-east-1".into(),
    access_key: std::env::var("AWS_ACCESS_KEY_ID")?,
    secret_key: std::env::var("AWS_SECRET_ACCESS_KEY")?,
    endpoint: None,   // Some("http://localhost:9000") for MinIO
    ..Default::default()
}).await?;
```

### Operations

```rust
// Upload
s3.put("data/file.json", bytes, Some("application/json")).await?;

// Download
let data: Bytes = s3.get("data/file.json").await?;

// Streaming upload (large files)
s3.put_stream("data/large.csv", stream, content_length).await?;

// List
let keys: Vec<String> = s3.list("data/").await?;

// Delete
s3.delete("data/file.json").await?;

// Presigned URL
let url = s3.presign_get("data/file.json", Duration::from_secs(3600)).await?;
```

### MinIO Configuration

```rust
S3Config {
    bucket: "my-bucket".into(),
    region: "us-east-1".into(),
    access_key: "minioadmin".into(),
    secret_key: "minioadmin".into(),
    endpoint: Some("http://localhost:9000".into()),
    path_style: true,   // required for MinIO
    ..Default::default()
}
```

### VilApp Integration

```rust
let service = ServiceProcess::new("files")
    .extension(s3.clone())
    .endpoint(Method::POST, "/upload", post(upload_file));

#[vil_handler(shm)]
async fn upload_file(ctx: ServiceCtx, slice: ShmSlice) -> VilResponse<String> {
    let s3 = ctx.state::<S3Connector>();
    let key = format!("uploads/{}", uuid::Uuid::new_v4());
    s3.put(&key, slice.bytes(), None).await?;
    VilResponse::ok(key)
}
```

## GCS (vil_conn_gcs)

```rust
use vil_conn_gcs::{GcsConnector, GcsConfig};

let gcs = GcsConnector::new(GcsConfig {
    bucket: "my-bucket".into(),
    credentials_json: std::fs::read_to_string("service_account.json")?,
    ..Default::default()
}).await?;

// Operations mirror S3
gcs.put("path/file.json", bytes, None).await?;
let data = gcs.get("path/file.json").await?;
let url = gcs.presign_get("path/file.json", Duration::from_secs(3600)).await?;
```

## Azure Blob (vil_conn_azure_blob)

```rust
use vil_conn_azure_blob::{AzureBlobConnector, AzureBlobConfig};

let azure = AzureBlobConnector::new(AzureBlobConfig {
    account: "mystorageaccount".into(),
    container: "my-container".into(),
    access_key: std::env::var("AZURE_STORAGE_KEY")?,
    ..Default::default()
}).await?;

azure.put("blobs/file.json", bytes, None).await?;
let data = azure.get("blobs/file.json").await?;
```

## Log Integration

All storage operations auto-emit `system_log!` entries via `#[connector_event]`. To observe:

```rust
// Events emitted automatically:
// connector.s3.put  { key, bytes, duration_us }
// connector.s3.get  { key, bytes, duration_us }
// connector.s3.error { key, error, kind: "fault" }
```

See [macros.md](macros.md) for `#[connector_fault/event/state]` details.

## Common Patterns

### Streaming download to pipeline
```rust
let stream = s3.get_stream("data/large.ndjson").await?;
let source = HttpSourceBuilder::from_stream(stream).ndjson();
```

### Upload pipeline output
```rust
#[vil_handler(shm)]
async fn process_and_store(ctx: ServiceCtx, slice: ShmSlice) -> VilResponse<()> {
    let result = transform(slice.json::<Record>()?);
    let s3 = ctx.state::<S3Connector>();
    s3.put(&format!("results/{}.json", result.id), result.to_bytes(), None).await?;
    VilResponse::ok(())
}
```
