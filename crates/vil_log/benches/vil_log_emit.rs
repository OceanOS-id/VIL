// =============================================================================
// benches/vil_log_emit.rs — vil_log app_log! throughput with NullDrain
// =============================================================================
//
// Measures the overhead of `app_log!` macro:
//   - timestamp read (SystemTime)
//   - fxhash of module_path
//   - rmp_serde::to_vec_named of a small BTreeMap
//   - try_push into SPSC ring
//
// Reports ns/event.
// =============================================================================

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use vil_log::app_log;
use vil_log::emit::ring::init_ring;

fn setup_ring() {
    // OnceLock — will silently no-op on second call in the same process.
    // Use std::sync::OnceLock trick via a static flag.
    static INIT: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    INIT.get_or_init(|| {
        init_ring(8192);
    });
}

fn bench_app_log(c: &mut Criterion) {
    setup_ring();

    let mut group = c.benchmark_group("vil_log_emit");
    group.throughput(Throughput::Elements(1));
    group.sample_size(500);

    group.bench_function("app_log_single_event", |b| {
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

criterion_group!(benches, bench_app_log);
criterion_main!(benches);
