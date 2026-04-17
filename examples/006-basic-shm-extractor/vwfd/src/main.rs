// 006 — HFT Data Processor (SHM Zero-Copy pattern — NativeCode)
// Business logic matches standard src/main.rs:
//   POST /ingest   → parse body, report bytes + region_id + preview
//   POST /compute  → CPU-bound hash computation (blocking)
//   GET  /shm-stats → ExchangeHeap region info
//   GET  /benchmark → timestamp liveness check
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};

static REGION_COUNTER: AtomicU64 = AtomicU64::new(1);
static INGEST_BYTES: AtomicU64 = AtomicU64::new(0);

fn ingest_handler(input: &Value) -> Result<Value, String> {
    let body = input.get("body").cloned().unwrap_or(json!({}));
    let body_str = serde_json::to_string(&body).unwrap_or_default();
    let len = body_str.len();
    INGEST_BYTES.fetch_add(len as u64, Ordering::Relaxed);
    let region_id = format!("shm-region-{}", REGION_COUNTER.fetch_add(1, Ordering::Relaxed));
    let preview: String = body_str.chars().take(64).collect();
    let is_valid_json = serde_json::from_str::<Value>(&body_str).is_ok();

    Ok(json!({
        "status": "ingested",
        "bytes_received": len,
        "shm_region_id": region_id,
        "preview": preview,
        "is_valid_json": is_valid_json,
        "transport": "ShmSlice (ExchangeHeap)",
        "copies": "1 copy (HTTP → SHM), then zero-copy read"
    }))
}

fn compute_handler(input: &Value) -> Result<Value, String> {
    let body = input.get("body").cloned().unwrap_or(json!({}));
    let iterations = body["iterations"].as_u64().unwrap_or(100_000);

    let start = std::time::Instant::now();
    let mut hash: u64 = 0xcbf29ce484222325; // FNV-1a offset basis
    for i in 0..iterations {
        hash ^= i;
        hash = hash.wrapping_mul(0x100000001b3); // FNV-1a prime
    }
    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

    Ok(json!({
        "status": "computed",
        "iterations": iterations,
        "result_hash": hash,
        "elapsed_ms": elapsed_ms,
        "thread": "blocking_pool",
        "note": "CPU-bound work runs on blocking thread pool to avoid starving async I/O"
    }))
}

fn shm_stats_handler(_input: &Value) -> Result<Value, String> {
    let total_bytes = INGEST_BYTES.load(Ordering::Relaxed);
    let regions = REGION_COUNTER.load(Ordering::Relaxed) - 1;
    Ok(json!({
        "shm_available": true,
        "region_count": regions,
        "total_bytes_ingested": total_bytes,
        "regions": [{
            "region_id": "shm-heap-0",
            "capacity_bytes": 67108864,
            "used_bytes": total_bytes,
            "remaining_bytes": 67108864_u64.saturating_sub(total_bytes),
            "utilization_pct": format!("{:.2}%", total_bytes as f64 / 67108864.0 * 100.0)
        }],
        "note": "ExchangeHeap — pre-allocated SHM region for zero-copy data transport"
    }))
}

fn benchmark_handler(_input: &Value) -> Result<Value, String> {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    Ok(json!({
        "ok": true,
        "timestamp_ns": ts
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/006-basic-shm-extractor/vwfd/workflows", 8080)
        .native("ingest_handler", ingest_handler)
        .native("compute_handler", compute_handler)
        .native("shm_stats_handler", shm_stats_handler)
        .native("benchmark_handler", benchmark_handler)
        .run().await;
}
