// =============================================================================
// vil_log::runtime — Drain runtime / init_logging
// =============================================================================
//
// `init_logging(config, drain)` sets up:
//   1. The global SPSC ring (init_ring)
//   2. A tokio background task that drains the ring into the drain
//
// The drain loop:
//   - Polls the ring on a short interval
//   - Accumulates a batch up to `config.batch_size`
//   - Calls `drain.flush(batch)` when batch is full OR interval elapsed
//   - On shutdown (via CancellationToken or process exit), calls drain.shutdown()
// =============================================================================

use std::time::Duration;

use tokio::time::{interval, MissedTickBehavior};

use crate::config::LogConfig;
use crate::drain::traits::LogDrain;
use crate::emit::ring::init_ring;
use crate::types::LogSlot;

/// Initialize the VIL log system.
///
/// - Initializes the global ring with `config.ring_slots` capacity.
/// - Spawns a tokio task that periodically drains the ring into `drain`.
///
/// # Panics
/// Panics if called more than once (ring init is one-shot).
pub fn init_logging<D>(config: LogConfig, drain: D) -> tokio::task::JoinHandle<()>
where
    D: LogDrain + 'static,
{
    crate::emit::ring::set_global_level(config.level);
    init_ring(config.ring_slots, config.threads);
    spawn_drain_task(config, drain)
}

/// Spawn just the drain task (ring must already be initialized).
pub fn spawn_drain_task<D>(config: LogConfig, mut drain: D) -> tokio::task::JoinHandle<()>
where
    D: LogDrain + 'static,
{
    let batch_size        = config.batch_size.max(1);
    let flush_interval_ms = config.flush_interval_ms.max(1);

    tokio::spawn(async move {
        let striped = crate::emit::ring::global_striped();
        let mut tick = interval(Duration::from_millis(flush_interval_ms));
        tick.set_missed_tick_behavior(MissedTickBehavior::Skip);

        let mut batch: Vec<LogSlot> = Vec::with_capacity(batch_size);

        loop {
            tick.tick().await;

            // Drain from all 4 striped rings
            let n = striped.drain_all(&mut batch, batch_size);

            if n > 0 {
                if let Err(e) = drain.flush(&batch).await {
                    eprintln!("vil_log drain '{}' error: {}", drain.name(), e);
                }
                batch.clear();
            }
        }
    })
}

/// Initialize logging with the global tracing subscriber.
///
/// Installs `VilTracingLayer` as the global tracing subscriber in addition
/// to setting up the drain task.
pub fn init_logging_with_tracing<D>(config: LogConfig, drain: D) -> tokio::task::JoinHandle<()>
where
    D: LogDrain + 'static,
{
    use tracing_subscriber::prelude::*;
    use crate::emit::VilTracingLayer;

    let subscriber = tracing_subscriber::registry()
        .with(VilTracingLayer::new());

    // Best-effort: ignore if already set.
    let _ = tracing::subscriber::set_global_default(subscriber);

    init_logging(config, drain)
}
