use std::sync::OnceLock;
use std::time::Instant;

static START_TIME: OnceLock<Instant> = OnceLock::new();

/// Returns a nanosecond timestamp since runtime start (inter-process safe on the same host).
pub fn now_ns() -> u64 {
    let start = START_TIME.get_or_init(Instant::now);
    Instant::now().duration_since(*start).as_nanos() as u64
}
