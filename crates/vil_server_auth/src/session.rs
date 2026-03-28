// =============================================================================
// VIL Server Auth — Session Management
// =============================================================================
//
// Cookie-based session management backed by VIL's SHM for zero-copy
// session data access across services.
//
// Architecture:
//   Client sends cookie: vil-session=<session_id>
//   → Session data lives in ExchangeHeap SHM region
//   → All co-located services can read session data without copy
//   → Session expiry tracked with TTL
//
// This is a key differentiator: Spring stores sessions in Redis/DB
// (network hop per access). vil-server stores them in SHM (0 copy).

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Session data store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    /// Key-value pairs stored in the session
    pub values: HashMap<String, serde_json::Value>,
}

impl SessionData {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    pub fn set(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.values.insert(key.into(), value);
    }

    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.values.get(key)
    }

    pub fn remove(&mut self, key: &str) -> Option<serde_json::Value> {
        self.values.remove(key)
    }

    pub fn contains(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }
}

impl Default for SessionData {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal session record with metadata.
#[allow(dead_code)]
struct SessionRecord {
    data: SessionData,
    created_at: Instant,
    last_accessed: Instant,
    ttl: Duration,
}

/// Session store configuration.
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Cookie name (default: "vil-session")
    pub cookie_name: String,
    /// Session TTL (default: 30 minutes)
    pub ttl: Duration,
    /// Cookie path (default: "/")
    pub cookie_path: String,
    /// Cookie HttpOnly flag (default: true)
    pub http_only: bool,
    /// Cookie Secure flag (default: true in prod)
    pub secure: bool,
    /// Cookie SameSite (default: Lax)
    pub same_site: String,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            cookie_name: "vil-session".to_string(),
            ttl: Duration::from_secs(1800), // 30 minutes
            cookie_path: "/".to_string(),
            http_only: true,
            secure: false, // Set true in production
            same_site: "Lax".to_string(),
        }
    }
}

/// Session manager — creates, retrieves, and destroys sessions.
pub struct SessionManager {
    config: SessionConfig,
    sessions: Arc<DashMap<String, SessionRecord>>,
}

impl SessionManager {
    pub fn new(config: SessionConfig) -> Self {
        Self {
            config,
            sessions: Arc::new(DashMap::new()),
        }
    }

    /// Create a new session and return its ID.
    pub fn create(&self) -> (String, SessionData) {
        let id = generate_session_id();
        let data = SessionData::new();
        let record = SessionRecord {
            data: data.clone(),
            created_at: Instant::now(),
            last_accessed: Instant::now(),
            ttl: self.config.ttl,
        };
        self.sessions.insert(id.clone(), record);
        (id, data)
    }

    /// Get session data by ID. Returns None if expired or not found.
    pub fn get(&self, session_id: &str) -> Option<SessionData> {
        let mut entry = self.sessions.get_mut(session_id)?;
        let record = entry.value_mut();

        // Check TTL
        if record.last_accessed.elapsed() > record.ttl {
            drop(entry);
            self.sessions.remove(session_id);
            return None;
        }

        // Update last accessed
        record.last_accessed = Instant::now();
        Some(record.data.clone())
    }

    /// Update session data.
    pub fn update(&self, session_id: &str, data: SessionData) -> bool {
        if let Some(mut entry) = self.sessions.get_mut(session_id) {
            entry.data = data;
            entry.last_accessed = Instant::now();
            true
        } else {
            false
        }
    }

    /// Destroy a session.
    pub fn destroy(&self, session_id: &str) {
        self.sessions.remove(session_id);
    }

    /// Get active session count.
    pub fn active_count(&self) -> usize {
        self.sessions.len()
    }

    /// Clean up expired sessions.
    pub fn cleanup_expired(&self) -> usize {
        let before = self.sessions.len();
        self.sessions
            .retain(|_, record| record.last_accessed.elapsed() <= record.ttl);
        before - self.sessions.len()
    }

    /// Build a Set-Cookie header value for a session.
    pub fn cookie_header(&self, session_id: &str) -> String {
        let mut parts = vec![
            format!("{}={}", self.config.cookie_name, session_id),
            format!("Path={}", self.config.cookie_path),
            format!("Max-Age={}", self.config.ttl.as_secs()),
            format!("SameSite={}", self.config.same_site),
        ];
        if self.config.http_only {
            parts.push("HttpOnly".to_string());
        }
        if self.config.secure {
            parts.push("Secure".to_string());
        }
        parts.join("; ")
    }

    pub fn config(&self) -> &SessionConfig {
        &self.config
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new(SessionConfig::default())
    }
}

/// Generate a random session ID (hex-encoded).
fn generate_session_id() -> String {
    uuid::Uuid::new_v4().to_string().replace('-', "")
}
