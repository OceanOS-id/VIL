# 601-storage-s3-basic

Simple S3 put / get / list with `vil_log` resolved output.

## What it shows

- `S3Client::new()` with MinIO-compatible config
- `put_object`, `get_object`, `list_objects`
- `db_log!` auto-emitted by `vil_storage_s3` on every operation
- `StdoutDrain::resolved()` output format

## Prerequisites

MinIO (or any S3-compatible endpoint):

```bash
docker run -p 9000:9000 -p 9001:9001 \
  -e MINIO_ROOT_USER=minioadmin \
  -e MINIO_ROOT_PASSWORD=minioadmin \
  minio/minio server /data --console-address :9001
```

Create the bucket first:

```bash
mc alias set local http://localhost:9000 minioadmin minioadmin
mc mb local/vil-demo
```

## Run

```bash
cargo run -p example-601-storage-s3-basic
```

Without MinIO, the example prints the config and exits gracefully.
