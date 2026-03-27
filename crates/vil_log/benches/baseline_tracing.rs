// =============================================================================
// benches/baseline_tracing.rs — Baseline: tracing::info! throughput
// =============================================================================
//
// Measures the raw overhead of `tracing::info!` with a NonBlocking appender
// writing to /dev/null. Reports ns/event.
// =============================================================================

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use tracing_appender::non_blocking;

fn bench_tracing_info(c: &mut Criterion) {
    // Set up a non-blocking tracing subscriber writing to /dev/null
    let (writer, _guard) = non_blocking(std::io::sink());

    use tracing_subscriber::fmt;
    let subscriber = fmt::Subscriber::builder()
        .with_writer(writer)
        .with_ansi(false)
        .finish();

    // Best effort; ignore if already set in another bench binary
    let _ = tracing::subscriber::set_global_default(subscriber);

    let mut group = c.benchmark_group("baseline_tracing");
    group.throughput(Throughput::Elements(1));
    group.sample_size(500);

    group.bench_function("tracing_info_single_event", |b| {
        b.iter(|| {
            tracing::info!(
                target: "benchmark",
                user_id = black_box(42u64),
                action  = black_box("login"),
                success = black_box(true),
            );
        });
    });

    group.finish();
}

criterion_group!(benches, bench_tracing_info);
criterion_main!(benches);
