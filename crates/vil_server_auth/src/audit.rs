// =============================================================================
// VIL Server Auth — Audit Logging Middleware
// =============================================================================
//
// Records security-relevant events for compliance and forensics.
// Events are emitted as structured logs via tracing and optionally
// written to a dedicated audit log store.
//
// Tracked events:
//   - Authentication attempts (success/failure)
//   - Authorization denials
//   - Rate limit triggers
//   - Circuit breaker state changes
//   - Admin endpoint access
//   - CSRF validation failures
//   - API key usage

use serde::Serialize;
use std::sync::Arc;
use std::time::SystemTime;

/// Audit event type.
#[derive(Debug, Clone, Serialize)]
pub enum AuditEventType {
    AuthSuccess,
    AuthFailure,
    AuthorizationDenied,
    RateLimited,
    CircuitBreakerTripped,
    AdminAccess,
    CsrfFailure,
    ApiKeyUsed,
    ApiKeyRevoked,
    SessionCreated,
    SessionDestroyed,
    ConfigChanged,
}

/// Audit event record.
#[derive(Debug, Clone, Serialize)]
pub struct AuditEvent {
    pub timestamp: u64,
    pub event_type: AuditEventType,
    pub actor: String,
    pub resource: String,
    pub action: String,
    pub outcome: String,
    pub ip_address: Option<String>,
    pub request_id: Option<String>,
    pub details: Option<String>,
}

impl AuditEvent {
    pub fn new(event_type: AuditEventType, actor: &str, resource: &str, action: &str) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            timestamp,
            event_type,
            actor: actor.to_string(),
            resource: resource.to_string(),
            action: action.to_string(),
            outcome: "success".to_string(),
            ip_address: None,
            request_id: None,
            details: None,
        }
    }

    pub fn outcome(mut self, outcome: &str) -> Self {
        self.outcome = outcome.to_string();
        self
    }

    pub fn ip(mut self, ip: &str) -> Self {
        self.ip_address = Some(ip.to_string());
        self
    }

    pub fn request_id(mut self, id: &str) -> Self {
        self.request_id = Some(id.to_string());
        self
    }

    pub fn details(mut self, details: &str) -> Self {
        self.details = Some(details.to_string());
        self
    }
}

/// Audit log store — collects and persists audit events.
pub struct AuditLog {
    /// In-memory event buffer (ring buffer)
    events: Arc<std::sync::RwLock<Vec<AuditEvent>>>,
    /// Maximum events to keep in memory
    max_events: usize,
}

impl AuditLog {
    pub fn new(max_events: usize) -> Self {
        Self {
            events: Arc::new(std::sync::RwLock::new(Vec::with_capacity(max_events))),
            max_events,
        }
    }

    /// Record an audit event.
    pub fn record(&self, event: AuditEvent) {
        // Emit as structured log
        tracing::info!(
            event_type = ?event.event_type,
            actor = %event.actor,
            resource = %event.resource,
            action = %event.action,
            outcome = %event.outcome,
            ip = ?event.ip_address,
            request_id = ?event.request_id,
            "audit"
        );

        // Store in ring buffer
        let mut events = self.events.write().unwrap();
        if events.len() >= self.max_events {
            events.remove(0); // Ring buffer eviction
        }
        events.push(event);
    }

    /// Get recent audit events.
    pub fn recent(&self, limit: usize) -> Vec<AuditEvent> {
        let events = self.events.read().unwrap();
        let start = events.len().saturating_sub(limit);
        events[start..].to_vec()
    }

    /// Get total event count.
    pub fn count(&self) -> usize {
        self.events.read().unwrap().len()
    }

    /// Clear all events.
    pub fn clear(&self) {
        self.events.write().unwrap().clear();
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new(10000)
    }
}
