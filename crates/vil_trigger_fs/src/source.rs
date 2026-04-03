// =============================================================================
// vil_trigger_fs::source — FsTrigger: TriggerSource impl
// =============================================================================
//
// Watches a filesystem path with the `notify` crate (inotify/FSEvents/kqueue)
// and fires `TriggerEvent` descriptors on matching events.
//
// Tri-Lane (§5):
//   Trigger Lane ← TriggerEvent descriptor on every matched FS event.
//   Data Lane    ← Not used (path is stored as a hash, not payload).
//   Control Lane ← stop() / pause() / resume() from the pipeline.
//
// Semantic log (§8):
//   Every fire emits `mq_log!(Info, MqPayload { ... })` with timing.
//   No println!, tracing::info!, log::info!.
//
// Note on notify v7:
//   notify 7.x exposes `RecommendedWatcher` + `RecursiveMode` from
//   `notify::RecursiveMode` and `notify::Watcher`.  The crate uses
//   `notify::event::EventKind` to distinguish create/modify/delete/rename.
// =============================================================================

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::{mpsc, watch};

use vil_log::dict::register_str;
use vil_log::{mq_log, MqPayload};
use vil_trigger_core::{EventCallback, TriggerEvent, TriggerFault, TriggerSource};

use crate::config::FsConfig;

// =============================================================================
// FsTrigger
// =============================================================================

/// Trigger source that fires on filesystem events.
pub struct FsTrigger {
    /// User-provided configuration.
    config: FsConfig,

    /// Channel to deliver events to downstream consumers.
    tx: mpsc::Sender<TriggerEvent>,

    /// Monotonic sequence counter.
    sequence: Arc<AtomicU64>,

    /// Pause flag — when set, events are silently dropped.
    paused: Arc<AtomicBool>,

    /// Cancellation signal.
    cancel_tx: Option<watch::Sender<bool>>,
}

impl FsTrigger {
    /// Construct a new `FsTrigger`.
    pub fn new(config: FsConfig, tx: mpsc::Sender<TriggerEvent>) -> Self {
        register_str(config.watch_path);
        if let Some(pat) = config.pattern {
            register_str(pat);
        }
        register_str("fs");

        Self {
            config,
            tx,
            sequence: Arc::new(AtomicU64::new(0)),
            paused: Arc::new(AtomicBool::new(false)),
            cancel_tx: None,
        }
    }

    /// Convert a notify `EventKind` to a compact op-code (used in fs_loop).
    pub(crate) fn event_op_static(kind: &EventKind) -> u8 {
        match kind {
            EventKind::Create(_) => 0,
            EventKind::Modify(_) => 1,
            EventKind::Remove(_) => 2,
            _ => 255,
        }
    }
}

// =============================================================================
// TriggerSource impl
// =============================================================================

#[async_trait]
impl TriggerSource for FsTrigger {
    fn kind(&self) -> &'static str {
        "fs"
    }

    async fn start(&self, on_event: EventCallback) -> Result<(), TriggerFault> {
        let trigger_id = self.config.trigger_id;
        let watch_path = self.config.watch_path;
        let recursive = self.config.recursive;
        let debounce_ms = self.config.debounce_ms;
        let sequence = Arc::clone(&self.sequence);
        let paused = Arc::clone(&self.paused);
        let tx = self.tx.clone();

        let (cancel_tx, cancel_rx) = watch::channel(false);

        // SAFETY: same as in CronTrigger — start/stop are not concurrent.
        unsafe {
            let self_mut = self as *const FsTrigger as *mut FsTrigger;
            (*self_mut).cancel_tx = Some(cancel_tx);
        }

        // Build a sync std::mpsc channel for notify callbacks (notify is sync).
        let (notify_tx, notify_rx) = std::sync::mpsc::channel::<notify::Result<Event>>();

        let mut watcher = RecommendedWatcher::new(
            notify_tx,
            notify::Config::default().with_poll_interval(Duration::from_millis(debounce_ms)),
        )
        .map_err(|_| TriggerFault::SourceUnavailable {
            kind_hash: register_str("fs"),
            reason_code: 1,
        })?;

        let mode = if recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };

        watcher.watch(Path::new(watch_path), mode).map_err(|_| {
            TriggerFault::SourceUnavailable {
                kind_hash: register_str("fs"),
                reason_code: 2,
            }
        })?;

        // Clone the event-mask config fields for the spawned task.
        let events_mask = self.config.events;
        let pattern = self.config.pattern;

        tokio::spawn(fs_loop(
            trigger_id,
            watch_path,
            events_mask,
            pattern,
            sequence,
            paused,
            tx,
            on_event,
            notify_rx,
            cancel_rx,
            watcher,
        ));

        Ok(())
    }

    async fn pause(&self) -> Result<(), TriggerFault> {
        self.paused.store(true, Ordering::Relaxed);
        Ok(())
    }

    async fn resume(&self) -> Result<(), TriggerFault> {
        self.paused.store(false, Ordering::Relaxed);
        Ok(())
    }

    async fn stop(&self) -> Result<(), TriggerFault> {
        // SAFETY: same as start().
        unsafe {
            let self_mut = self as *const FsTrigger as *mut FsTrigger;
            if let Some(ref cancel_tx) = (*self_mut).cancel_tx {
                let _ = cancel_tx.send(true);
            }
            (*self_mut).cancel_tx = None;
        }
        Ok(())
    }
}

// =============================================================================
// Background filesystem-watch task
// =============================================================================

#[allow(clippy::too_many_arguments)]
async fn fs_loop(
    trigger_id: u64,
    watch_path: &'static str,
    events_mask: crate::config::FsEventMask,
    pattern: Option<&'static str>,
    sequence: Arc<AtomicU64>,
    paused: Arc<AtomicBool>,
    tx: mpsc::Sender<TriggerEvent>,
    on_event: EventCallback,
    notify_rx: std::sync::mpsc::Receiver<notify::Result<Event>>,
    mut cancel_rx: watch::Receiver<bool>,
    // Keep watcher alive — dropping it stops the OS subscription.
    _watcher: RecommendedWatcher,
) {
    let kind_hash = register_str("fs");
    let path_hash = register_str(watch_path);
    let broker_hash = register_str("fs");

    loop {
        // Check cancellation without blocking.
        if cancel_rx.has_changed().unwrap_or(false) {
            if *cancel_rx.borrow_and_update() {
                break;
            }
        }

        // Poll the sync notify channel with a short timeout so we can check
        // the cancel signal periodically.
        let event_result = tokio::task::spawn_blocking({
            let notify_rx_ptr = &notify_rx as *const _ as usize;
            move || {
                // SAFETY: the Receiver is pinned to this task for its lifetime.
                let rx = unsafe {
                    &*(notify_rx_ptr as *const std::sync::mpsc::Receiver<notify::Result<Event>>)
                };
                rx.recv_timeout(Duration::from_millis(100))
            }
        })
        .await;

        let event = match event_result {
            Ok(Ok(Ok(e))) => e,
            Ok(Ok(Err(_notify_err))) => {
                // Notify error — log and continue.
                mq_log!(
                    Info,
                    MqPayload {
                        broker_hash,
                        topic_hash: register_str("trigger.fs.notify_error"),
                        offset: trigger_id,
                        op_type: 3, // nack
                        ..MqPayload::default()
                    }
                );
                continue;
            }
            Ok(Err(_timeout)) => continue, // Timeout — loop to check cancel.
            Err(_join_err) => break,       // Task panicked — exit.
        };

        if paused.load(Ordering::Relaxed) {
            continue;
        }

        // Apply event kind filter.
        let op = FsTrigger::event_op_static(&event.kind);
        if !filter_event(&event.kind, events_mask) {
            continue;
        }

        // Apply optional glob pattern filter.
        if let Some(pat) = pattern {
            let matches = event
                .paths
                .iter()
                .any(|p| p.to_str().map(|s| glob_match(pat, s)).unwrap_or(false));
            if !matches {
                continue;
            }
        }

        // --- Fire ---
        let fire_start = Instant::now();
        let fire_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        let seq = sequence.fetch_add(1, Ordering::Relaxed);

        let trig_event = TriggerEvent {
            kind_hash,
            source_hash: path_hash,
            sequence: seq,
            timestamp_ns: fire_ns,
            payload_bytes: 0,
            op,
            _pad: [0; 3],
        };

        on_event(trig_event);
        let _ = tx.try_send(trig_event);

        let elapsed_ns = fire_start.elapsed().as_nanos() as u64;

        mq_log!(
            Info,
            MqPayload {
                broker_hash,
                topic_hash: path_hash,
                offset: seq,
                e2e_latency_ns: elapsed_ns,
                op_type: 0, // publish
                ..MqPayload::default()
            }
        );
    }
}

// =============================================================================
// Helpers
// =============================================================================

fn filter_event(kind: &EventKind, mask: crate::config::FsEventMask) -> bool {
    match kind {
        EventKind::Create(_) => mask.on_create,
        EventKind::Modify(_) => mask.on_modify,
        EventKind::Remove(_) => mask.on_delete,
        _ => false,
    }
}

/// Minimal glob matcher: supports `*` (any chars) and `?` (single char).
/// For production use, replace with the `glob` crate.
fn glob_match(pattern: &str, input: &str) -> bool {
    // Simple recursive implementation — acceptable for low-frequency trigger use.
    let pat: Vec<char> = pattern.chars().collect();
    let inp: Vec<char> = input.chars().collect();
    glob_inner(&pat, &inp)
}

fn glob_inner(pat: &[char], inp: &[char]) -> bool {
    match (pat.first(), inp.first()) {
        (None, None) => true,
        (Some('*'), _) => {
            // Star matches zero or more characters.
            glob_inner(&pat[1..], inp) || (!inp.is_empty() && glob_inner(pat, &inp[1..]))
        }
        (Some('?'), Some(_)) => glob_inner(&pat[1..], &inp[1..]),
        (Some(p), Some(i)) if p == i => glob_inner(&pat[1..], &inp[1..]),
        _ => false,
    }
}
