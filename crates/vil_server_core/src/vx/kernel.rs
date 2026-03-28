//! VX Kernel Executor — Token-based non-blocking execution loop
//!
//! The kernel executor drives the Tri-Lane message processing for VX services.
//! It receives messages from Trigger/Data/Control lanes and dispatches them
//! to endpoint handlers, tracking per-request state via tokens.
//!
//! Design principles:
//! - CPU work separated from I/O wait (single await point)
//! - Token-based state machine per request
//! - Non-blocking: exhaust ready queue before parking
//! - Control Lane independent from Data Lane (no head-of-line blocking)

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::collections::VecDeque;
use dashmap::DashMap;

/// Token state for a single in-flight request
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenState {
    /// Waiting for trigger (initial state)
    Pending,
    /// In ready queue, waiting to be dispatched
    Ready,
    /// Dispatched to handler, awaiting completion
    Active,
    /// Handler completed successfully
    Completed,
    /// Handler failed with error
    Failed,
    /// Request was cancelled/aborted
    Cancelled,
}

impl std::fmt::Display for TokenState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Ready => write!(f, "ready"),
            Self::Active => write!(f, "active"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// A token representing one in-flight request in the kernel
pub struct ExecutionToken {
    /// Unique request ID
    pub request_id: u64,
    /// Current state
    pub state: TokenState,
    /// Service this request is targeting
    pub service: String,
    /// Timestamp when token was created (nanos)
    pub created_at: u64,
    /// Timestamp when handler started (nanos, 0 if not started)
    pub started_at: u64,
    /// Timestamp when handler completed (nanos, 0 if not done)
    pub completed_at: u64,
    /// Payload data (raw bytes from Trigger Lane)
    pub payload: Vec<u8>,
}

/// Kernel metrics — atomic counters for observability
pub struct KernelMetrics {
    /// Total requests received
    pub total_received: AtomicU64,
    /// Total requests completed successfully
    pub total_completed: AtomicU64,
    /// Total requests failed
    pub total_failed: AtomicU64,
    /// Total requests cancelled
    pub total_cancelled: AtomicU64,
    /// Current in-flight count
    pub in_flight: AtomicUsize,
    /// Total control signals processed
    pub control_signals: AtomicU64,
}

impl KernelMetrics {
    pub fn new() -> Self {
        Self {
            total_received: AtomicU64::new(0),
            total_completed: AtomicU64::new(0),
            total_failed: AtomicU64::new(0),
            total_cancelled: AtomicU64::new(0),
            in_flight: AtomicUsize::new(0),
            control_signals: AtomicU64::new(0),
        }
    }

    /// Snapshot current metrics as a JSON-friendly struct
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            total_received: self.total_received.load(Ordering::Relaxed),
            total_completed: self.total_completed.load(Ordering::Relaxed),
            total_failed: self.total_failed.load(Ordering::Relaxed),
            total_cancelled: self.total_cancelled.load(Ordering::Relaxed),
            in_flight: self.in_flight.load(Ordering::Relaxed),
            control_signals: self.control_signals.load(Ordering::Relaxed),
        }
    }
}

impl Default for KernelMetrics {
    fn default() -> Self { Self::new() }
}

/// Immutable snapshot of kernel metrics
#[derive(Debug, Clone, serde::Serialize)]
pub struct MetricsSnapshot {
    pub total_received: u64,
    pub total_completed: u64,
    pub total_failed: u64,
    pub total_cancelled: u64,
    pub in_flight: usize,
    pub control_signals: u64,
}

/// Control signal types for the Control Lane
#[derive(Debug, Clone)]
pub enum ControlSignal {
    /// Request completed successfully
    Done { request_id: u64 },
    /// Request failed with error
    Error { request_id: u64, code: u16, reason: String },
    /// Abort request immediately
    Abort { request_id: u64 },
    /// Backpressure: pause accepting new requests
    Pause,
    /// Backpressure: resume accepting requests
    Resume,
    /// Health degraded signal
    HealthDegraded { reason: String },
    /// Health restored signal
    HealthRestored,
}

/// The VX Kernel — manages token lifecycle and dispatches work
///
/// The kernel operates in a loop:
/// 1. Exhaust ready queue (no await — pure CPU work)
/// 2. Check completion (all done? deadlock?)
/// 3. Park and wait (single await point: select! on lanes)
pub struct VxKernel {
    /// Service name this kernel manages
    service: String,
    /// Token table: request_id -> token
    tokens: DashMap<u64, ExecutionToken>,
    /// Ready queue: tokens waiting to be dispatched
    ready_queue: std::sync::Mutex<VecDeque<u64>>,
    /// Metrics
    metrics: Arc<KernelMetrics>,
    /// Whether the kernel is accepting new requests
    accepting: std::sync::atomic::AtomicBool,
}

impl VxKernel {
    /// Create a new kernel for a service
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            tokens: DashMap::new(),
            ready_queue: std::sync::Mutex::new(VecDeque::new()),
            metrics: Arc::new(KernelMetrics::new()),
            accepting: std::sync::atomic::AtomicBool::new(true),
        }
    }

    /// Get the service name
    pub fn service(&self) -> &str { &self.service }

    /// Get metrics reference
    pub fn metrics(&self) -> &Arc<KernelMetrics> { &self.metrics }

    /// Check if kernel is accepting requests
    pub fn is_accepting(&self) -> bool {
        self.accepting.load(Ordering::Relaxed)
    }

    /// Pause accepting (backpressure)
    pub fn pause(&self) { self.accepting.store(false, Ordering::Relaxed); }

    /// Resume accepting
    pub fn resume(&self) { self.accepting.store(true, Ordering::Relaxed); }

    /// Enqueue a new request token
    pub fn enqueue(&self, request_id: u64, service: String, payload: Vec<u8>) -> bool {
        if !self.is_accepting() {
            return false;
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        let token = ExecutionToken {
            request_id,
            state: TokenState::Ready,
            service,
            created_at: now,
            started_at: 0,
            completed_at: 0,
            payload,
        };

        self.tokens.insert(request_id, token);
        self.ready_queue.lock().unwrap().push_back(request_id);
        self.metrics.total_received.fetch_add(1, Ordering::Relaxed);
        self.metrics.in_flight.fetch_add(1, Ordering::Relaxed);
        true
    }

    /// Dequeue next ready token (non-blocking)
    pub fn dequeue_ready(&self) -> Option<u64> {
        let id = self.ready_queue.lock().unwrap().pop_front()?;
        if let Some(mut token) = self.tokens.get_mut(&id) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64;
            token.state = TokenState::Active;
            token.started_at = now;
        }
        Some(id)
    }

    /// Mark a token as completed
    pub fn complete(&self, request_id: u64) {
        if let Some(mut token) = self.tokens.get_mut(&request_id) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64;
            token.state = TokenState::Completed;
            token.completed_at = now;
        }
        self.metrics.total_completed.fetch_add(1, Ordering::Relaxed);
        self.metrics.in_flight.fetch_sub(1, Ordering::Relaxed);
    }

    /// Mark a token as failed
    pub fn fail(&self, request_id: u64) {
        if let Some(mut token) = self.tokens.get_mut(&request_id) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64;
            token.state = TokenState::Failed;
            token.completed_at = now;
        }
        self.metrics.total_failed.fetch_add(1, Ordering::Relaxed);
        self.metrics.in_flight.fetch_sub(1, Ordering::Relaxed);
    }

    /// Cancel a token (e.g., client disconnect)
    pub fn cancel(&self, request_id: u64) {
        if let Some(mut token) = self.tokens.get_mut(&request_id) {
            token.state = TokenState::Cancelled;
        }
        self.metrics.total_cancelled.fetch_add(1, Ordering::Relaxed);
        self.metrics.in_flight.fetch_sub(1, Ordering::Relaxed);
    }

    /// Get token state
    pub fn token_state(&self, request_id: u64) -> Option<TokenState> {
        self.tokens.get(&request_id).map(|t| t.state)
    }

    /// Get token payload
    pub fn token_payload(&self, request_id: u64) -> Option<Vec<u8>> {
        self.tokens.get(&request_id).map(|t| t.payload.clone())
    }

    /// Get current in-flight count
    pub fn in_flight(&self) -> usize {
        self.metrics.in_flight.load(Ordering::Relaxed)
    }

    /// Get ready queue length
    pub fn ready_count(&self) -> usize {
        self.ready_queue.lock().unwrap().len()
    }

    /// Process a control signal
    pub fn handle_control(&self, signal: ControlSignal) {
        self.metrics.control_signals.fetch_add(1, Ordering::Relaxed);
        match signal {
            ControlSignal::Done { request_id } => self.complete(request_id),
            ControlSignal::Error { request_id, .. } => self.fail(request_id),
            ControlSignal::Abort { request_id } => self.cancel(request_id),
            ControlSignal::Pause => self.pause(),
            ControlSignal::Resume => self.resume(),
            ControlSignal::HealthDegraded { reason } => {
                {
                    use vil_log::app_log;
                    app_log!(Warn, "vx.health.degraded", { service: self.service.as_str(), reason: reason.as_str() });
                }
            }
            ControlSignal::HealthRestored => {
                {
                    use vil_log::app_log;
                    app_log!(Info, "vx.health.restored", { service: self.service.as_str() });
                }
            }
        }
    }

    /// Clean up completed/failed/cancelled tokens older than max_age_nanos
    pub fn cleanup(&self, max_age_nanos: u64) -> usize {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        let mut cleaned = 0;
        let mut to_remove = Vec::new();

        for entry in self.tokens.iter() {
            let token = entry.value();
            match token.state {
                TokenState::Completed | TokenState::Failed | TokenState::Cancelled => {
                    if now - token.created_at > max_age_nanos {
                        to_remove.push(*entry.key());
                    }
                }
                _ => {}
            }
        }

        for id in to_remove {
            self.tokens.remove(&id);
            cleaned += 1;
        }

        cleaned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enqueue_and_dequeue() {
        let kernel = VxKernel::new("test-svc");

        // Enqueue
        assert!(kernel.enqueue(1, "test-svc".into(), b"payload-1".to_vec()));
        assert_eq!(kernel.ready_count(), 1);
        assert_eq!(kernel.token_state(1), Some(TokenState::Ready));

        // Dequeue transitions to Active
        let id = kernel.dequeue_ready().unwrap();
        assert_eq!(id, 1);
        assert_eq!(kernel.token_state(1), Some(TokenState::Active));
        assert_eq!(kernel.ready_count(), 0);

        // No more in queue
        assert!(kernel.dequeue_ready().is_none());
    }

    #[test]
    fn complete_and_metrics() {
        let kernel = VxKernel::new("test-svc");
        kernel.enqueue(1, "test-svc".into(), b"data".to_vec());
        kernel.dequeue_ready();
        kernel.complete(1);

        assert_eq!(kernel.token_state(1), Some(TokenState::Completed));
        assert_eq!(kernel.in_flight(), 0);

        let snap = kernel.metrics().snapshot();
        assert_eq!(snap.total_received, 1);
        assert_eq!(snap.total_completed, 1);
        assert_eq!(snap.total_failed, 0);
    }

    #[test]
    fn fail_and_cancel() {
        let kernel = VxKernel::new("test-svc");

        // Fail path
        kernel.enqueue(1, "test-svc".into(), b"a".to_vec());
        kernel.dequeue_ready();
        kernel.fail(1);
        assert_eq!(kernel.token_state(1), Some(TokenState::Failed));

        // Cancel path
        kernel.enqueue(2, "test-svc".into(), b"b".to_vec());
        kernel.dequeue_ready();
        kernel.cancel(2);
        assert_eq!(kernel.token_state(2), Some(TokenState::Cancelled));

        let snap = kernel.metrics().snapshot();
        assert_eq!(snap.total_failed, 1);
        assert_eq!(snap.total_cancelled, 1);
        assert_eq!(snap.in_flight, 0);
    }

    #[test]
    fn backpressure_pause_resume() {
        let kernel = VxKernel::new("test-svc");
        assert!(kernel.is_accepting());

        // Pause rejects enqueue
        kernel.pause();
        assert!(!kernel.is_accepting());
        assert!(!kernel.enqueue(1, "test-svc".into(), b"rejected".to_vec()));
        assert_eq!(kernel.token_state(1), None);

        // Resume accepts again
        kernel.resume();
        assert!(kernel.is_accepting());
        assert!(kernel.enqueue(2, "test-svc".into(), b"accepted".to_vec()));
        assert_eq!(kernel.token_state(2), Some(TokenState::Ready));
    }

    #[test]
    fn control_signal_processing() {
        let kernel = VxKernel::new("test-svc");

        // Setup tokens
        kernel.enqueue(10, "test-svc".into(), b"a".to_vec());
        kernel.enqueue(11, "test-svc".into(), b"b".to_vec());
        kernel.enqueue(12, "test-svc".into(), b"c".to_vec());
        kernel.dequeue_ready(); // 10
        kernel.dequeue_ready(); // 11
        kernel.dequeue_ready(); // 12

        // Done signal
        kernel.handle_control(ControlSignal::Done { request_id: 10 });
        assert_eq!(kernel.token_state(10), Some(TokenState::Completed));

        // Error signal
        kernel.handle_control(ControlSignal::Error {
            request_id: 11,
            code: 500,
            reason: "internal".into(),
        });
        assert_eq!(kernel.token_state(11), Some(TokenState::Failed));

        // Abort signal
        kernel.handle_control(ControlSignal::Abort { request_id: 12 });
        assert_eq!(kernel.token_state(12), Some(TokenState::Cancelled));

        // Pause/Resume
        kernel.handle_control(ControlSignal::Pause);
        assert!(!kernel.is_accepting());
        kernel.handle_control(ControlSignal::Resume);
        assert!(kernel.is_accepting());

        // Health signals (just verify they don't panic)
        kernel.handle_control(ControlSignal::HealthDegraded { reason: "test".into() });
        kernel.handle_control(ControlSignal::HealthRestored);

        let snap = kernel.metrics().snapshot();
        assert_eq!(snap.control_signals, 7);
    }

    #[test]
    fn cleanup_old_tokens() {
        let kernel = VxKernel::new("test-svc");

        kernel.enqueue(1, "test-svc".into(), b"old".to_vec());
        kernel.dequeue_ready();
        kernel.complete(1);

        // Cleanup with 0 max_age should remove completed token immediately
        let cleaned = kernel.cleanup(0);
        assert_eq!(cleaned, 1);
        assert_eq!(kernel.token_state(1), None);

        // Active tokens should not be cleaned
        kernel.enqueue(2, "test-svc".into(), b"active".to_vec());
        kernel.dequeue_ready();
        let cleaned = kernel.cleanup(0);
        assert_eq!(cleaned, 0);
        assert_eq!(kernel.token_state(2), Some(TokenState::Active));
    }

    #[test]
    fn metrics_snapshot() {
        let kernel = VxKernel::new("test-svc");
        kernel.enqueue(1, "test-svc".into(), b"x".to_vec());
        kernel.dequeue_ready();
        kernel.complete(1);

        let snap = kernel.metrics().snapshot();

        // Verify it's serializable
        let json = serde_json::to_string(&snap).unwrap();
        assert!(json.contains("\"total_received\":1"));
        assert!(json.contains("\"total_completed\":1"));
        assert!(json.contains("\"in_flight\":0"));
    }

    #[test]
    fn ready_queue_ordering() {
        let kernel = VxKernel::new("test-svc");

        // Enqueue in order
        kernel.enqueue(100, "test-svc".into(), b"first".to_vec());
        kernel.enqueue(200, "test-svc".into(), b"second".to_vec());
        kernel.enqueue(300, "test-svc".into(), b"third".to_vec());

        // FIFO order preserved
        assert_eq!(kernel.dequeue_ready(), Some(100));
        assert_eq!(kernel.dequeue_ready(), Some(200));
        assert_eq!(kernel.dequeue_ready(), Some(300));
        assert_eq!(kernel.dequeue_ready(), None);

        // Verify payloads
        assert_eq!(kernel.token_payload(100).unwrap(), b"first");
        assert_eq!(kernel.token_payload(200).unwrap(), b"second");
        assert_eq!(kernel.token_payload(300).unwrap(), b"third");
    }
}
