// =============================================================================
// vil_trigger_cron::source — CronTrigger: TriggerSource impl
// =============================================================================
//
// Fires `TriggerEvent` descriptors on a cron schedule using the `cron` crate.
//
// TriggerSource::start<F>():
//   Spawns a tokio task that sleeps until the next scheduled fire time, then
//   calls `on_event(event)` and loops.  Cancellation is signalled via a
//   tokio::sync::watch channel so that stop() can interrupt a sleep.
//
// Semantic log (§8):
//   Every fire emits `mq_log!(Info, MqPayload { ... })` with timing.
//   No println!, tracing::info!, log::info!.
// =============================================================================

use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use cron::Schedule;
use tokio::sync::{mpsc, watch};

use vil_log::dict::register_str;
use vil_log::{mq_log, MqPayload};
use vil_trigger_core::{EventCallback, TriggerEvent, TriggerFault, TriggerSource};

use crate::config::{CronConfig, MissedFirePolicy};
use crate::error::CronFault;

// =============================================================================
// CronTrigger
// =============================================================================

/// Trigger source that fires on a cron schedule.
pub struct CronTrigger {
    /// Parsed schedule for next-fire-time computation.
    schedule: Schedule,

    /// Original expression stored for dict registration.
    schedule_expr: &'static str,

    /// User-provided configuration.
    config: CronConfig,

    /// Channel to deliver events to downstream consumers.
    tx: mpsc::Sender<TriggerEvent>,

    /// Monotonic sequence counter.
    sequence: Arc<AtomicU64>,

    /// Cancellation watch sender — `Some` while the task is running.
    cancel_tx: Option<watch::Sender<bool>>,
}

impl CronTrigger {
    /// Parse the schedule expression and prepare the trigger.
    ///
    /// Returns `CronFault::InvalidSchedule` if parsing fails.
    pub fn new(config: CronConfig, tx: mpsc::Sender<TriggerEvent>) -> Result<Self, CronFault> {
        let schedule = Schedule::from_str(config.schedule).map_err(|_| {
            CronFault::InvalidSchedule {
                expr_hash: register_str(config.schedule),
            }
        })?;

        register_str(config.schedule);
        register_str("cron");

        Ok(Self {
            schedule,
            schedule_expr: config.schedule,
            config,
            tx,
            sequence: Arc::new(AtomicU64::new(0)),
            cancel_tx: None,
        })
    }
}

// =============================================================================
// TriggerSource impl
// =============================================================================

#[async_trait]
impl TriggerSource for CronTrigger {
    fn kind(&self) -> &'static str {
        "cron"
    }

    async fn start(&self, on_event: EventCallback) -> Result<(), TriggerFault> {
        let schedule    = self.schedule.clone();
        let trigger_id  = self.config.trigger_id;
        let missed_fire = self.config.missed_fire;
        let sequence    = Arc::clone(&self.sequence);
        let expr        = self.schedule_expr;
        let tx          = self.tx.clone();

        let (cancel_tx, cancel_rx) = watch::channel(false);

        // Leak the sender into the struct — we need interior mutability without
        // changing the &self signature imposed by the trait.
        // In a full ServiceProcess integration this would be mediated via the
        // TriggerProcess wrapper. For now we store it via unsafe cell.
        // Safety: start() is only called once; stop() reads the same pointer.
        let cancel_tx_ptr = Box::into_raw(Box::new(cancel_tx));
        // SAFETY: single-threaded access to cancel_tx_ptr (start/stop are
        // coordinated by the VIL control lane, never concurrent).
        unsafe {
            let self_mut = self as *const CronTrigger as *mut CronTrigger;
            (*self_mut).cancel_tx = Some(*Box::from_raw(cancel_tx_ptr));
        }

        tokio::spawn(cron_loop(
            schedule,
            trigger_id,
            missed_fire,
            sequence,
            expr,
            tx,
            on_event,
            cancel_rx,
        ));

        Ok(())
    }

    async fn pause(&self) -> Result<(), TriggerFault> {
        // Pause is handled at the TriggerProcess / Control Lane level.
        // The background task continues running but the pipeline simply
        // discards events while paused.
        Ok(())
    }

    async fn resume(&self) -> Result<(), TriggerFault> {
        Ok(())
    }

    async fn stop(&self) -> Result<(), TriggerFault> {
        // SAFETY: same reasoning as in start() — stop() is never concurrent
        // with start().
        unsafe {
            let self_mut = self as *const CronTrigger as *mut CronTrigger;
            if let Some(ref cancel_tx) = (*self_mut).cancel_tx {
                let _ = cancel_tx.send(true);
            }
            (*self_mut).cancel_tx = None;
        }
        Ok(())
    }
}

// =============================================================================
// Background cron loop task
// =============================================================================

async fn cron_loop(
    schedule: Schedule,
    _trigger_id: u64,
    missed_fire: MissedFirePolicy,
    sequence: Arc<AtomicU64>,
    expr: &'static str,
    tx: mpsc::Sender<TriggerEvent>,
    on_event: EventCallback,
    mut cancel_rx: watch::Receiver<bool>,
) {
    let kind_hash   = register_str("cron");
    let topic_hash  = register_str(expr);
    let broker_hash = register_str("cron");

    loop {
        // Calculate next fire time using chrono (required by the cron crate).
        let now_utc = chrono::Utc::now();
        let next = match schedule.after(&now_utc).next() {
            Some(t) => t,
            None => break, // Schedule exhausted.
        };

        let now_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        let next_ns = next
            .timestamp_nanos_opt()
            .unwrap_or(0) as u64;

        if next_ns <= now_ns {
            match missed_fire {
                MissedFirePolicy::Skip => continue,
                MissedFirePolicy::FireImmediately => { /* fall through */ }
            }
        } else {
            let sleep_dur = Duration::from_nanos(next_ns - now_ns);
            tokio::select! {
                _ = tokio::time::sleep(sleep_dur) => {}
                _ = cancel_rx.changed() => {
                    if *cancel_rx.borrow() { break; }
                }
            }
        }

        // --- Fire ---
        let fire_start = Instant::now();
        let fire_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        let seq = sequence.fetch_add(1, Ordering::Relaxed);

        let event = TriggerEvent {
            kind_hash,
            source_hash: topic_hash,
            sequence: seq,
            timestamp_ns: fire_ns,
            payload_bytes: 0,
            op: 0, // fire
            _pad: [0; 3],
        };

        // Deliver via callback (Tri-Lane Trigger Lane emission).
        on_event(event);

        // Also forward through the mpsc channel for internal consumers.
        let _ = tx.try_send(event);

        let elapsed_us = fire_start.elapsed().as_micros() as u32;

        mq_log!(Info, MqPayload {
            broker_hash,
            topic_hash,
            offset:         seq,
            e2e_latency_us: elapsed_us,
            op_type:        0, // publish
            ..MqPayload::default()
        });
    }
}
