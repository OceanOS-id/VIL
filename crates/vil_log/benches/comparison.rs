// =============================================================================
// benches/comparison.rs — Side-by-side comparison table
// =============================================================================
//
// Runs both baseline_tracing and vil_log_emit in one criterion group,
// producing a comparison table in the HTML report.
// =============================================================================

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use tracing_appender::non_blocking;
use vil_log::app_log;
use vil_log::emit::ring::init_ring;

fn setup_ring() {
    static INIT: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    INIT.get_or_init(|| {
        init_ring(8192);
    });
}

fn setup_tracing() {
    static INIT: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    INIT.get_or_init(|| {
        let (writer, _guard) = non_blocking(std::io::sink());
        // Guard is intentionally leaked to keep the writer alive for the benchmark
        std::mem::forget(_guard);
        use tracing_subscriber::fmt;
        let subscriber = fmt::Subscriber::builder()
            .with_writer(writer)
            .with_ansi(false)
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);
    });
}

fn comparison_benchmark(c: &mut Criterion) {
    setup_ring();
    setup_tracing();

    let mut group = c.benchmark_group("comparison");
    group.throughput(Throughput::Elements(1));
    group.sample_size(500);

    // Baseline: tracing::info!
    group.bench_function("tracing_info", |b| {
        b.iter(|| {
            tracing::info!(
                target: "benchmark",
                user_id = black_box(42u64),
                action  = black_box("login"),
                success = black_box(true),
            );
        });
    });

    // VIL: app_log! → SPSC ring (NullDrain-equivalent: ring fills and drops
    // since no consumer is running, but push overhead is measured)
    group.bench_function("vil_app_log", |b| {
        b.iter(|| {
            app_log!(Info, "bench.event", {
                user_id: black_box(42u64),
                action:  black_box("login"),
                success: black_box(true),
            });
        });
    });

    group.finish();
}

criterion_group!(benches, comparison_benchmark);
criterion_main!(benches);
