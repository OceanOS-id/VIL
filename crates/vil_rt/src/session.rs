// =============================================================================
// vil_rt::session — Generic Reactive Session Fabric
// =============================================================================
// Extracted from vil_http sink.rs Stage 2, generalized as core primitive.
//
// This module provides:
// - SessionEntry<T>: stateful mailbox per session (data + control channels)
// - PendingSlot<T>: bounded TTL buffer for early-arrival messages
// - SessionRegistry<T>: concurrent registry for session lifecycle management
//
// Adapter writers (vil_new_http, etc.) should use SessionRegistry
// instead of building their own session fabric.
// =============================================================================

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use vil_types::ControlSignal;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Default timeout for session inactivity.
const DEFAULT_SESSION_TIMEOUT: Duration = Duration::from_secs(10);

/// Default TTL for pending slots (early-arrival buffer).
const DEFAULT_PENDING_TTL: Duration = Duration::from_secs(3);

/// Maximum data messages buffered before session registration.
const DEFAULT_PENDING_MAX_DATA: usize = 32;

/// Maximum control signals buffered before session registration.
const DEFAULT_PENDING_MAX_CTRL: usize = 4;

// ---------------------------------------------------------------------------
// SessionEntry — per-session stateful mailbox
// ---------------------------------------------------------------------------

/// Stateful mailbox for a single session.
///
/// Contains separate channels for data and control signals,
/// ensuring control plane (DONE/ERROR/ABORT) is never blocked by data.
pub struct SessionEntry<T: Send + Sync + 'static> {
    /// Data channel — hot-path payload delivery.
    pub data_tx: tokio::sync::mpsc::UnboundedSender<T>,
    /// Control channel — out-of-band signals.
    pub ctrl_tx: tokio::sync::mpsc::UnboundedSender<ControlSignal>,
    /// When this session was created.
    pub created_at: Instant,
    /// Whether the first data message has been seen.
    pub first_seen: AtomicBool,
    /// Whether the session has been closed (DONE/ABORT received).
    pub closed: AtomicBool,
}

impl<T: Send + Sync + 'static> SessionEntry<T> {
    /// Create a new session entry with fresh channels.
    pub fn new(
        data_tx: tokio::sync::mpsc::UnboundedSender<T>,
        ctrl_tx: tokio::sync::mpsc::UnboundedSender<ControlSignal>,
    ) -> Self {
        Self {
            data_tx,
            ctrl_tx,
            created_at: Instant::now(),
            first_seen: AtomicBool::new(false),
            closed: AtomicBool::new(false),
        }
    }

    /// Mark that the first data message has been received.
    pub fn mark_first_seen(&self) {
        self.first_seen.store(true, Ordering::Release);
    }

    /// Mark the session as closed.
    pub fn mark_closed(&self) {
        self.closed.store(true, Ordering::Release);
    }

    /// Check if the session has been closed.
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }

    /// Check how long since session creation.
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }
}

// ---------------------------------------------------------------------------
// PendingSlot — early-arrival buffer
// ---------------------------------------------------------------------------

/// Bounded buffer for messages that arrive before session registration.
///
/// Prevents first-message loss due to race between session registration
/// and data/control delivery. Has a TTL to avoid unbounded accumulation.
pub struct PendingSlot<T: Send + Sync + 'static> {
    /// Buffered data messages.
    pub data: VecDeque<T>,
    /// Buffered control signals.
    pub ctrl: VecDeque<ControlSignal>,
    /// When this slot was created.
    pub created_at: Instant,
}

impl<T: Send + Sync + 'static> PendingSlot<T> {
    pub fn new() -> Self {
        Self {
            data: VecDeque::with_capacity(8),
            ctrl: VecDeque::with_capacity(2),
            created_at: Instant::now(),
        }
    }

    /// Check if this pending slot has expired.
    pub fn is_expired(&self, ttl: Duration) -> bool {
        self.created_at.elapsed() > ttl
    }

    /// Reset the slot (clear data/ctrl, reset timestamp).
    pub fn reset(&mut self) {
        self.data.clear();
        self.ctrl.clear();
        self.created_at = Instant::now();
    }
}

impl<T: Send + Sync + 'static> Default for PendingSlot<T> {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// SessionRegistry — concurrent session lifecycle manager
// ---------------------------------------------------------------------------

/// Configuration for a SessionRegistry.
#[derive(Clone, Debug)]
pub struct SessionConfig {
    pub session_timeout: Duration,
    pub pending_ttl: Duration,
    pub pending_max_data: usize,
    pub pending_max_ctrl: usize,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            session_timeout: DEFAULT_SESSION_TIMEOUT,
            pending_ttl: DEFAULT_PENDING_TTL,
            pending_max_data: DEFAULT_PENDING_MAX_DATA,
            pending_max_ctrl: DEFAULT_PENDING_MAX_CTRL,
        }
    }
}

/// Concurrent session registry for reactive adapters.
///
/// Manages session lifecycle: registration, data/control delivery,
/// pending buffer flush, and cleanup.
///
/// Generic over `T`: the data message type flowing through the data lane.
pub struct SessionRegistry<T: Send + Sync + 'static> {
    sessions: Arc<DashMap<u64, Arc<SessionEntry<T>>>>,
    pending: Arc<DashMap<u64, PendingSlot<T>>>,
    config: SessionConfig,
}

impl<T: Send + Sync + 'static> SessionRegistry<T> {
    /// Create a new registry with default configuration.
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
            pending: Arc::new(DashMap::new()),
            config: SessionConfig::default(),
        }
    }

    /// Create a new registry with custom configuration.
    pub fn with_config(config: SessionConfig) -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
            pending: Arc::new(DashMap::new()),
            config,
        }
    }

    /// Register a new session. Returns (data_rx, ctrl_rx) for the caller to consume.
    pub fn register(
        &self,
        session_id: u64,
    ) -> (
        tokio::sync::mpsc::UnboundedReceiver<T>,
        tokio::sync::mpsc::UnboundedReceiver<ControlSignal>,
    ) {
        let (data_tx, data_rx) = tokio::sync::mpsc::unbounded_channel();
        let (ctrl_tx, ctrl_rx) = tokio::sync::mpsc::unbounded_channel();

        let entry = Arc::new(SessionEntry::new(data_tx, ctrl_tx));
        self.sessions.insert(session_id, entry.clone());

        // Flush any pending messages that arrived before registration
        self.flush_pending(session_id, &entry);

        (data_rx, ctrl_rx)
    }

    /// Deliver a data message to a session. Buffers if session not yet registered.
    pub fn deliver_data(&self, session_id: u64, msg: T) -> bool {
        // Try active session first
        if let Some(entry_ref) = self.sessions.get(&session_id) {
            let entry = entry_ref.value().clone();
            drop(entry_ref);
            entry.mark_first_seen();
            return entry.data_tx.send(msg).is_ok();
        }

        // Buffer in pending slot
        let mut slot = self
            .pending
            .entry(session_id)
            .or_insert_with(PendingSlot::new);
        if slot.is_expired(self.config.pending_ttl) {
            slot.reset();
        }
        if slot.data.len() < self.config.pending_max_data {
            slot.data.push_back(msg);
        } else {
            // Ring buffer: drop oldest
            let _ = slot.data.pop_front();
            slot.data.push_back(msg);
        }
        false
    }

    /// Deliver a control signal to a session. Buffers if session not yet registered.
    pub fn deliver_control(&self, session_id: u64, signal: ControlSignal) -> bool {
        // Try active session first
        if let Some(entry_ref) = self.sessions.get(&session_id) {
            let entry = entry_ref.value().clone();
            drop(entry_ref);
            if signal.is_terminal() {
                entry.mark_closed();
            }
            return entry.ctrl_tx.send(signal).is_ok();
        }

        // Buffer in pending slot
        let mut slot = self
            .pending
            .entry(session_id)
            .or_insert_with(PendingSlot::new);
        if slot.is_expired(self.config.pending_ttl) {
            slot.reset();
        }
        if slot.ctrl.len() < self.config.pending_max_ctrl {
            slot.ctrl.push_back(signal);
        }
        false
    }

    /// Flush pending messages for a session into its active entry.
    fn flush_pending(&self, session_id: u64, entry: &Arc<SessionEntry<T>>) {
        if let Some((_, mut slot)) = self.pending.remove(&session_id) {
            while let Some(msg) = slot.data.pop_front() {
                entry.mark_first_seen();
                let _ = entry.data_tx.send(msg);
            }
            while let Some(ctrl) = slot.ctrl.pop_front() {
                if ctrl.is_terminal() {
                    entry.mark_closed();
                }
                let _ = entry.ctrl_tx.send(ctrl);
            }
        }
    }

    /// Remove a session and its pending slot.
    pub fn cleanup(&self, session_id: u64) {
        self.sessions.remove(&session_id);
        self.pending.remove(&session_id);
    }

    /// Get the current session count.
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Get the current pending slot count.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Get a reference to the sessions map (for dispatcher access).
    pub fn sessions(&self) -> &Arc<DashMap<u64, Arc<SessionEntry<T>>>> {
        &self.sessions
    }

    /// Get a reference to the pending map (for dispatcher access).
    pub fn pending(&self) -> &Arc<DashMap<u64, PendingSlot<T>>> {
        &self.pending
    }

    /// Get the session configuration.
    pub fn config(&self) -> &SessionConfig {
        &self.config
    }
}

impl<T: Send + Sync + 'static> Default for SessionRegistry<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Send + Sync + 'static> Clone for SessionRegistry<T> {
    fn clone(&self) -> Self {
        Self {
            sessions: self.sessions.clone(),
            pending: self.pending.clone(),
            config: self.config.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_register_and_deliver() {
        let registry = SessionRegistry::<String>::new();
        let (mut data_rx, _ctrl_rx) = registry.register(1);

        assert!(registry.deliver_data(1, "hello".to_string()));
        let msg = data_rx.recv().await.unwrap();
        assert_eq!(msg, "hello");
    }

    #[tokio::test]
    async fn test_pending_buffer_flush() {
        let registry = SessionRegistry::<String>::new();

        // Deliver before registration — should be buffered
        registry.deliver_data(42, "early_msg".to_string());
        registry.deliver_control(42, ControlSignal::done(42));

        // Now register — pending should flush
        let (mut data_rx, mut ctrl_rx) = registry.register(42);

        let msg = data_rx.recv().await.unwrap();
        assert_eq!(msg, "early_msg");

        let ctrl = ctrl_rx.recv().await.unwrap();
        assert_eq!(ctrl, ControlSignal::Done { session_id: 42 });
    }

    #[test]
    fn test_session_cleanup() {
        let registry = SessionRegistry::<String>::new();
        let _channels = registry.register(1);
        assert_eq!(registry.session_count(), 1);

        registry.cleanup(1);
        assert_eq!(registry.session_count(), 0);
    }

    #[test]
    fn test_pending_max_data() {
        let config = SessionConfig {
            pending_max_data: 2,
            ..Default::default()
        };
        let registry = SessionRegistry::<String>::with_config(config);

        registry.deliver_data(1, "a".into());
        registry.deliver_data(1, "b".into());
        registry.deliver_data(1, "c".into()); // should push out "a"

        let slot = registry.pending().get(&1).unwrap();
        assert_eq!(slot.data.len(), 2);
        assert_eq!(slot.data[0], "b");
        assert_eq!(slot.data[1], "c");
    }
}
