// =============================================================================
// example-507-villog-bench-file-drain — File Drain E2E Benchmark
// =============================================================================

use std::path::PathBuf;
use std::time::Instant;

use tracing_subscriber::layer::SubscriberExt;

use vil_log::drain::{FileDrain, RotationStrategy};
use vil_log::runtime::init_logging;
use vil_log::types::*;
use vil_log::{access_log, LogConfig, LogLevel};

const EVENTS: u32 = 500_000;

fn bench_tracing_file() -> (std::time::Duration, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.into_path();

    let file_appender = tracing_appender::rolling::never(&path, "tracing.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let subscriber = tracing_subscriber::registry().with(
        tracing_subscriber::fmt::layer()
            .with_writer(non_blocking)
            .with_ansi(false)
            .json(),
    );
    let _guard2 = tracing::subscriber::set_default(subscriber);

    let start = Instant::now();
    for i in 0..EVENTS {
        tracing::info!(
            counter = i,
            method = "POST",
            status = 200u16,
            latency_ns = 2300000u64,
            path = "/api/orders",
            "request completed"
        );
    }
    drop(_guard2);
    let dur = start.elapsed();
    (dur, path)
}

async fn bench_vil_file() -> (std::time::Duration, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.into_path();

    let drain = FileDrain::new(
        &path,
        "vil",
        RotationStrategy::Size {
            max_bytes: 100 * 1024 * 1024,
        }, // 100MB — no rotation during bench
        1,
    )
    .expect("Failed to create FileDrain");

    let config = LogConfig {
        ring_slots: 1 << 20,
        level: LogLevel::Trace,
        batch_size: 4096,
        flush_interval_ms: 1,
        threads: None,
        dict_path: None,
        fallback_path: None,
        drain_failure_threshold: 3,
    };
    let task = init_logging(config, drain);

    let start = Instant::now();
    for _i in 0..EVENTS {
        access_log!(
            Info,
            AccessPayload {
                method: 1,
                status_code: 200,
                protocol: 0,
                duration_ns: 2_300_000,
                request_bytes: 256,
                response_bytes: 1024,
                client_ip: 0x7F000001,
                server_port: 8080,
                route_hash: 0x1234,
                user_agent_hash: 0x5678,
                path_hash: 0xABCD,
                session_id: 99999,
                authenticated: 1,
                cache_status: 0,
                _pad: [0; 14],
            }
        );
    }
    let emit_dur = start.elapsed();

    // Wait for drain to flush
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    task.abort();

    (emit_dur, path)
}

fn file_size(dir: &std::path::Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                total += meta.len();
            }
        }
    }
    total
}

fn print_row(name: &str, dur: std::time::Duration, size: u64) {
    let ns = dur.as_nanos() as f64 / EVENTS as f64;
    let mps = EVENTS as f64 / dur.as_secs_f64() / 1_000_000.0;
    let mb = size as f64 / 1024.0 / 1024.0;
    println!(
        "  {:<42} {:>6} {:>10.1} {:>8.2} {:>8.1}",
        name,
        dur.as_millis(),
        ns,
        mps,
        mb
    );
}

#[tokio::main]
async fn main() {
    println!();
    println!("  ╔══════════════════════════════════════════════════════════════════════════╗");
    println!("  ║        VIL Log vs tracing — File Drain Benchmark                        ║");
    println!(
        "  ║        {} events | --release | writing to /tmp                   ║",
        EVENTS
    );
    println!("  ╚══════════════════════════════════════════════════════════════════════════╝");
    println!();

    let (t_dur, t_path) = bench_tracing_file();
    let t_size = file_size(&t_path);

    let (v_dur, v_path) = bench_vil_file().await;
    let v_size = file_size(&v_path);

    println!(
        "  {:<42} {:>6} {:>10} {:>8} {:>8}",
        "Benchmark", "ms", "ns/event", "M ev/s", "MB"
    );
    println!("  {}", "═".repeat(76));
    print_row("tracing (JSON fmt + rolling file)", t_dur, t_size);
    print_row("VIL access_log! → FileDrain (JSON Lines)", v_dur, v_size);
    println!("  {}", "═".repeat(76));

    let speedup = t_dur.as_nanos() as f64 / v_dur.as_nanos() as f64;
    println!();
    if speedup >= 1.0 {
        println!("  Emit throughput: VIL is {:.1}x faster", speedup);
    } else {
        println!("  Emit throughput: tracing is {:.1}x faster", 1.0 / speedup);
    }
    println!();
    println!("  Note: VIL emit time measures only the hot path (push to ring).");
    println!("  Actual file writes happen async on the drain thread.");
    println!("  tracing emit time includes formatting + channel send.");

    let _ = std::fs::remove_dir_all(&t_path);
    let _ = std::fs::remove_dir_all(&v_path);
    println!();
}
