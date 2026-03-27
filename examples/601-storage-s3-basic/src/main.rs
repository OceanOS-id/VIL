// =============================================================================
// example-601-storage-s3-basic — S3 put / get / list with vil_log resolved
// =============================================================================
//
// Demonstrates:
//   - S3Client::new() with MinIO-compatible config
//   - put_object, get_object, list_objects
//   - db_log! auto-emitted by the crate on every operation
//   - StdoutDrain::resolved() output
//
// Requires: MinIO (or any S3-compatible endpoint) running locally.
// Quick start:
//   docker run -p 9000:9000 -e MINIO_ROOT_USER=minioadmin \
//     -e MINIO_ROOT_PASSWORD=minioadmin minio/minio server /data
//
// Without Docker, this example prints config and exits gracefully.
// =============================================================================

use bytes::Bytes;
use vil_log::drain::{StdoutDrain, StdoutFormat};
use vil_log::runtime::init_logging;
use vil_log::{LogConfig, LogLevel};
use vil_storage_s3::{S3Client, S3Config};

#[tokio::main]
async fn main() {
    // ── Init vil_log with resolved (human-readable) drain ──
    let config = LogConfig {
        ring_slots:        4096,
        level:             LogLevel::Info,
        batch_size:        64,
        flush_interval_ms: 50,
        threads:           None,
    };
    let _task = init_logging(config, StdoutDrain::new(StdoutFormat::Resolved));

    println!();
    println!("  example-601-storage-s3-basic");
    println!("  S3 put / get / list with vil_log resolved output");
    println!();

    let s3_cfg = S3Config {
        endpoint:   Some("http://localhost:9000".into()),
        region:     "us-east-1".into(),
        access_key: Some("minioadmin".into()),
        secret_key: Some("minioadmin".into()),
        bucket:     "vil-demo".into(),
        path_style: true,
    };

    println!("  Connecting to S3 endpoint: {}", s3_cfg.endpoint.as_deref().unwrap_or("AWS"));
    println!("  Bucket: {}", s3_cfg.bucket);
    println!();
    println!("  NOTE: Requires MinIO or S3-compatible endpoint.");
    println!("  Start with:");
    println!("    docker run -p 9000:9000 -p 9001:9001 \\");
    println!("      -e MINIO_ROOT_USER=minioadmin \\");
    println!("      -e MINIO_ROOT_PASSWORD=minioadmin \\");
    println!("      minio/minio server /data --console-address :9001");
    println!();

    let client = match S3Client::new(s3_cfg).await {
        Ok(c) => c,
        Err(e) => {
            println!("  [SKIP] Cannot connect to S3: {:?}", e);
            println!("  (All db_log! calls would appear above in resolved format)");
            return;
        }
    };

    // ── PUT ──
    let payload = Bytes::from("Hello from VIL example-601!");
    match client.put_object("demo/hello.txt", payload).await {
        Ok(res) => println!("  PUT  demo/hello.txt  etag={:?}", res.e_tag),
        Err(e)  => println!("  PUT  error: {:?}", e),
    }

    // ── GET ──
    match client.get_object("demo/hello.txt").await {
        Ok(data) => println!("  GET  demo/hello.txt  bytes={}", data.len()),
        Err(e)   => println!("  GET  error: {:?}", e),
    }

    // ── LIST ──
    match client.list_objects("demo/").await {
        Ok(objects) => {
            println!("  LIST demo/  count={}", objects.len());
            for obj in &objects {
                println!("       - {} ({} bytes)", obj.key, obj.size);
            }
        }
        Err(e) => println!("  LIST error: {:?}", e),
    }

    // Allow drain to flush
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    println!();
    println!("  Done. db_log! entries emitted above in resolved format.");
    println!();
}
