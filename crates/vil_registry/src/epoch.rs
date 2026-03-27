// =============================================================================
// vil_registry::epoch — Epoch Tracker
// =============================================================================
// EpochTracker manages per-process generation versions.
// Used for crash detection: if the epoch changes, the process has restarted.
//
// TASK LIST:
// [x] EpochTracker struct (per-process epoch management)
// [x] register / advance / current / is_alive
// [x] Unit tests
// [ ] TODO(future): atomic-based epoch for lock-free checks
// =============================================================================

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use vil_types::{Epoch, ProcessId};

/// Process state in the epoch tracker.
#[derive(Clone, Debug)]
pub struct ProcessEpochState {
    pub epoch: Epoch,
    pub alive: bool,
}

/// Epoch tracker: tracks generation and liveness of each process.
///
/// When a process crashes and restarts, its epoch advances. Consumers
/// holding an older epoch know that data from that epoch may be stale.
#[derive(Clone, Default)]
pub struct EpochTracker {
    inner: Arc<Mutex<HashMap<ProcessId, ProcessEpochState>>>,
}

impl EpochTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new process with epoch=1, alive=true.
    pub fn register(&self, process_id: ProcessId) {
        let mut guard = self.inner.lock().expect("epoch tracker lock poisoned");
        guard.insert(
            process_id,
            ProcessEpochState {
                epoch: Epoch(1),
                alive: true,
            },
        );
    }

    /// Advance the process epoch (e.g. on restart).
    pub fn advance(&self, process_id: ProcessId) {
        let mut guard = self.inner.lock().expect("epoch tracker lock poisoned");
        if let Some(state) = guard.get_mut(&process_id) {
            state.epoch = Epoch(state.epoch.0 + 1);
            state.alive = true; // restart -> alive again
        }
    }

    /// Get the current epoch for a process.
    pub fn current(&self, process_id: ProcessId) -> Option<Epoch> {
        let guard = self.inner.lock().expect("epoch tracker lock poisoned");
        guard.get(&process_id).map(|s| s.epoch)
    }

    /// Check whether a process is still alive.
    pub fn is_alive(&self, process_id: ProcessId) -> bool {
        let guard = self.inner.lock().expect("epoch tracker lock poisoned");
        guard.get(&process_id).is_some_and(|s| s.alive)
    }

    /// Mark a process as dead.
    pub fn mark_dead(&self, process_id: ProcessId) {
        let mut guard = self.inner.lock().expect("epoch tracker lock poisoned");
        if let Some(state) = guard.get_mut(&process_id) {
            state.alive = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_query() {
        let tracker = EpochTracker::new();
        let pid = ProcessId(1);

        tracker.register(pid);
        assert!(tracker.is_alive(pid));
        assert_eq!(tracker.current(pid), Some(Epoch(1)));
    }

    #[test]
    fn test_advance_epoch() {
        let tracker = EpochTracker::new();
        let pid = ProcessId(1);

        tracker.register(pid);
        tracker.advance(pid);
        assert_eq!(tracker.current(pid), Some(Epoch(2)));
    }

    #[test]
    fn test_mark_dead_and_restart() {
        let tracker = EpochTracker::new();
        let pid = ProcessId(1);

        tracker.register(pid);
        assert!(tracker.is_alive(pid));

        tracker.mark_dead(pid);
        assert!(!tracker.is_alive(pid));

        // Simulate restart by advancing epoch
        tracker.advance(pid);
        assert!(tracker.is_alive(pid));
        assert_eq!(tracker.current(pid), Some(Epoch(2)));
    }

    #[test]
    fn test_unknown_process() {
        let tracker = EpochTracker::new();
        assert!(!tracker.is_alive(ProcessId(999)));
        assert_eq!(tracker.current(ProcessId(999)), None);
    }
}
