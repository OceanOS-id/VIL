// =============================================================================
// vil_log::runtime — Drain runtime / init_logging
// =============================================================================
//
// `init_logging(config, drain)` sets up:
//   1. The global SPSC ring (init_ring)
//   2. A tokio background task that drains the ring into the drain
//   3. Dictionary auto-persistence (load on startup, save on shutdown)
//
// The drain loop:
//   - Polls the ring on a short interval
//   - Accumulates a batch up to `config.batch_size`
//   - Calls `drain.flush(batch)` when batch is full OR interval elapsed
//   - On shutdown (via CancellationToken or process exit), calls drain.shutdown()
// =============================================================================

use std::path::PathBuf;
use std::time::Duration;

use tokio::time::{interval, MissedTickBehavior};

use crate::config::LogConfig;
use crate::drain::traits::LogDrain;
use crate::emit::ring::init_ring;
use crate::types::LogSlot;

/// Initialize the VIL log system.
///
/// - Initializes the global ring with `config.ring_slots` capacity.
/// - Loads dictionary from file if it exists.
/// - Spawns a tokio task that periodically drains the ring into `drain`.
/// - Registers a shutdown handler to persist the dictionary on ctrl+c/SIGTERM.
///
/// # Panics
/// Panics if called more than once (ring init is one-shot).
pub fn init_logging<D>(config: LogConfig, drain: D) -> tokio::task::JoinHandle<()>
where
    D: LogDrain + 'static,
{
    crate::emit::ring::set_global_level(config.level);
    init_ring(config.ring_slots, config.threads);

    // Determine dictionary path
    let dict_path = config
        .dict_path
        .clone()
        .unwrap_or_else(|| PathBuf::from(".vil_log_dict.json"));

    // Load existing dict on startup
    if dict_path.exists() {
        match crate::dict::load_from_file(&dict_path) {
            Ok(n) => {
                if n > 0 {
                    eprintln!(
                        "[vil_log] Loaded {} dictionary entries from {:?}",
                        n, dict_path
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "[vil_log] Failed to load dictionary from {:?}: {}",
                    dict_path, e
                );
            }
        }
    }

    // Register shutdown handler to persist dictionary
    let dict_path_clone = dict_path.clone();
    tokio::spawn(async move {
        // Wait for ctrl+c
        let _ = tokio::signal::ctrl_c().await;
        if let Err(e) = crate::dict::save_to_file(&dict_path_clone) {
            eprintln!("[vil_log] Failed to save dictionary on shutdown: {}", e);
        }
    });

    spawn_drain_task(config, drain)
}

/// Spawn just the drain task (ring must already be initialized).
pub fn spawn_drain_task<D>(config: LogConfig, mut drain: D) -> tokio::task::JoinHandle<()>
where
    D: LogDrain + 'static,
{
    let batch_size = config.batch_size.max(1);
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
    use crate::emit::VilTracingLayer;
    use tracing_subscriber::prelude::*;

    let subscriber = tracing_subscriber::registry().with(VilTracingLayer::new());

    // Best-effort: ignore if already set.
    let _ = tracing::subscriber::set_global_default(subscriber);

    init_logging(config, drain)
}
