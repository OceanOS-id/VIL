// =============================================================================
// vil_obs::events — Trace Events
// =============================================================================
// All observable events in the VIL runtime. Each event carries
// a timestamp, identity, and relevant context.
//
// TASK LIST:
// [x] TraceEvent enum (Published, Received, Dropped, ProcessCrashed, etc.)
// [x] Timestamp helper
// =============================================================================

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use vil_types::{PortId, ProcessId, SampleId};

/// Timestamp in nanoseconds since the UNIX epoch.
pub fn now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

/// Observable event produced by the VIL runtime.
#[derive(Clone, Debug)]
pub enum TraceEvent {
    /// Sample successfully published to the queue.
    Published {
        ts_ns: u64,
        sample_id: SampleId,
        origin_port: PortId,
        owner: ProcessId,
        /// Monotonic instant for latency calculation.
        instant: Instant,
    },

    /// Sample received by a consumer from the queue.
    Received {
        ts_ns: u64,
        sample_id: SampleId,
        target_port: PortId,
        /// Publish-to-receive latency in nanoseconds (if available).
        latency_ns: Option<u64>,
    },

    /// Sample dropped (expired, rejected, or reclaimed).
    Dropped {
        ts_ns: u64,
        sample_id: SampleId,
        reason: DropReason,
    },

    /// Process crash detected and cleanup performed.
    ProcessCrashed {
        ts_ns: u64,
        process_id: ProcessId,
        orphan_count: usize,
        drained_count: usize,
    },

    /// Process shut down gracefully.
    ProcessShutdown { ts_ns: u64, process_id: ProcessId },

    /// Port connected.
    Connected {
        ts_ns: u64,
        from_port: PortId,
        to_port: PortId,
    },

    /// Queue depth snapshot (periodic sampling).
    QueueDepthSample {
        ts_ns: u64,
        port_id: PortId,
        depth: usize,
    },
}

/// Reason a sample was dropped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropReason {
    /// Timeout expired.
    Timeout,
    /// Queue full (backpressure drop).
    BackpressureDrop,
    /// Process crash — orphan reclaim.
    OrphanReclaim,
    /// Manual release by consumer.
    ManualRelease,
}

impl std::fmt::Display for TraceEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Published {
                sample_id,
                origin_port,
                owner,
                ..
            } => {
                write!(f, "PUBLISH {} from {} by {}", sample_id, origin_port, owner)
            }
            Self::Received {
                sample_id,
                target_port,
                latency_ns,
                ..
            } => {
                if let Some(lat) = latency_ns {
                    write!(
                        f,
                        "RECV {} at {} ({}µs)",
                        sample_id,
                        target_port,
                        lat / 1000
                    )
                } else {
                    write!(f, "RECV {} at {}", sample_id, target_port)
                }
            }
            Self::Dropped {
                sample_id, reason, ..
            } => {
                write!(f, "DROP {} ({:?})", sample_id, reason)
            }
            Self::ProcessCrashed {
                process_id,
                orphan_count,
                drained_count,
                ..
            } => {
                write!(
                    f,
                    "CRASH {} orphans={} drained={}",
                    process_id, orphan_count, drained_count
                )
            }
            Self::ProcessShutdown { process_id, .. } => {
                write!(f, "SHUTDOWN {}", process_id)
            }
            Self::Connected {
                from_port, to_port, ..
            } => {
                write!(f, "CONNECT {} → {}", from_port, to_port)
            }
            Self::QueueDepthSample { port_id, depth, .. } => {
                write!(f, "QDEPTH {} = {}", port_id, depth)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_now_ns() {
        let t = now_ns();
        assert!(t > 0);
    }

    #[test]
    fn test_event_display() {
        let ev = TraceEvent::Published {
            ts_ns: now_ns(),
            sample_id: SampleId(1),
            origin_port: PortId(2),
            owner: ProcessId(3),
            instant: Instant::now(),
        };
        let s = format!("{}", ev);
        assert!(s.contains("PUBLISH"));
        assert!(s.contains("Sample(1)"));
    }
}
