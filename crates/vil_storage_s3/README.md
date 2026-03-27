# vil_storage_s3

VIL Storage Plugin — S3-compatible object storage client for AWS S3, MinIO,
Cloudflare R2, DigitalOcean Spaces, and any service that implements the AWS S3 API.

## Features

- Put, get, delete, list, and head operations with full timing instrumentation
- Presigned URL generation for time-limited anonymous downloads
- MinIO / path-style addressing support
- Automatic `db_log!` emission after every operation (VIL Semantic Log §8)
- Zero heap-allocated strings on the log path — `register_str()` hashes only
- No `println!`, `tracing::info!`, or `eprintln!` — compliant with COMPLIANCE.md

---

## Quick Start

```rust
use vil_storage_s3::{S3Client, S3Config};
use bytes::Bytes;

#[tokio::main]
async fn main() {
    // --- MinIO (local development) ---
    let cfg = S3Config {
        endpoint: Some("http://localhost:9000".into()),
        region: "us-east-1".into(),
        access_key: Some("minioadmin".into()),
        secret_key: Some("minioadmin".into()),
        bucket: "my-bucket".into(),
        path_style: true,  // required for MinIO
    };

    let client = S3Client::new(cfg).await.expect("failed to build S3 client");

    // Upload
    client
        .put_object("docs/hello.txt", Bytes::from("hello, vil!"))
        .await
        .expect("put failed");

    // Download
    let body = client.get_object("docs/hello.txt").await.expect("get failed");
    println!("{}", String::from_utf8_lossy(&body));

    // List
    let objects = client.list_objects("docs/").await.expect("list failed");
    for obj in &objects {
        println!("{} ({} bytes)", obj.key, obj.size);
    }

    // Presigned URL (valid for 1 hour)
    let url = client
        .presigned_url("docs/hello.txt", 3600)
        .await
        .expect("presign failed");
    println!("presigned: {url}");

    // Delete
    client.delete_object("docs/hello.txt").await.expect("delete failed");
}
```

---

## S3Config Fields

| Field | Type | Description |
|-------|------|-------------|
| `endpoint` | `Option<String>` | Custom endpoint URL. `None` uses the default AWS S3 endpoint for the region. Set to `Some("http://localhost:9000")` for MinIO. |
| `region` | `String` | AWS region, e.g. `"us-east-1"`, `"ap-southeast-1"`, or `"auto"` (Cloudflare R2). |
| `access_key` | `Option<String>` | AWS access key ID. `None` resolves from environment (`AWS_ACCESS_KEY_ID`, `~/.aws/credentials`, IAM role). |
| `secret_key` | `Option<String>` | AWS secret access key. `None` resolves from environment alongside `access_key`. |
| `bucket` | `String` | Name of the bucket to operate on. |
| `path_style` | `bool` | Use path-style URLs (`endpoint/bucket/key`). **Must be `true` for MinIO.** |

---

## Supported Operations

| Method | Description | Log op_type |
|--------|-------------|-------------|
| `put_object(key, body)` | Upload bytes to a key | `1` (INSERT) |
| `get_object(key)` | Download bytes from a key | `0` (SELECT) |
| `delete_object(key)` | Delete a key (idempotent) | `3` (DELETE) |
| `list_objects(prefix)` | List keys sharing a prefix | `0` (SELECT) |
| `head_object(key)` | Fetch metadata without downloading | `0` (SELECT) |
| `presigned_url(key, secs)` | Generate a time-limited GET URL | `0` (SELECT) |

---

## Tri-Lane Mapping

| Lane | Direction | Content |
|------|-----------|---------|
| Trigger | Inbound → VIL | Upload / download request |
| Data | Bidirectional | Object content (streamed, chunked into SHM pages) |
| Control | Bidirectional | Progress, error, presigned URL response |

---

## Boundary Classification

| Path | Copy vs Zero-copy |
|------|-------------------|
| Network I/O (S3 wire protocol) | Copy — required by HTTP |
| Internal log payloads | Zero-copy (`DbPayload` is `Copy`, placed on ring) |
| Config / metadata at setup time | Copy — acceptable per COMPLIANCE.md §2 |

---

## Log Thread Budget

This crate spawns **no** internal threads. No adjustment to `LogConfig.threads` is needed.

---

## AWS S3 Example

```rust
use vil_storage_s3::{S3Client, S3Config};

let cfg = S3Config {
    endpoint: None,  // default AWS S3 endpoint
    region: "ap-southeast-1".into(),
    access_key: Some(std::env::var("AWS_ACCESS_KEY_ID").unwrap()),
    secret_key: Some(std::env::var("AWS_SECRET_ACCESS_KEY").unwrap()),
    bucket: "my-production-bucket".into(),
    path_style: false,
};

let client = S3Client::new(cfg).await?;
```

## Cloudflare R2 Example

```rust
use vil_storage_s3::{S3Client, S3Config};

let cfg = S3Config {
    endpoint: Some("https://<account-id>.r2.cloudflarestorage.com".into()),
    region: "auto".into(),
    access_key: Some(std::env::var("R2_ACCESS_KEY_ID").unwrap()),
    secret_key: Some(std::env::var("R2_SECRET_ACCESS_KEY").unwrap()),
    bucket: "my-r2-bucket".into(),
    path_style: false,
};

let client = S3Client::new(cfg).await?;
```
