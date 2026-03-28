// =============================================================================
// VIL Server — Error Tracker & Aggregator
// =============================================================================
//
// Tracks and aggregates errors across all handlers.
// Groups errors by pattern (status code + path + error type) for
// quick identification of systemic issues.

use dashmap::DashMap;
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::SystemTime;

/// A tracked error occurrence.
#[derive(Debug, Clone, Serialize)]
pub struct ErrorRecord {
    pub timestamp: u64,
    pub method: String,
    pub path: String,
    pub status_code: u16,
    pub error_message: String,
    pub request_id: Option<String>,
}

/// Error pattern — groups similar errors.
#[derive(Debug, Clone, Serialize)]
pub struct ErrorPattern {
    pub pattern_key: String,
    pub first_seen: u64,
    pub last_seen: u64,
    pub count: u64,
    pub sample_message: String,
}

/// Error tracker — collects, aggregates, and reports errors.
pub struct ErrorTracker {
    /// Recent errors (ring buffer)
    errors: Arc<std::sync::RwLock<Vec<ErrorRecord>>>,
    /// Error patterns (aggregated)
    patterns: DashMap<String, ErrorPattern>,
    /// Total error count
    total: AtomicU64,
    /// Max recent errors to keep
    max_recent: usize,
}

impl ErrorTracker {
    pub fn new(max_recent: usize) -> Self {
        Self {
            errors: Arc::new(std::sync::RwLock::new(Vec::with_capacity(max_recent))),
            patterns: DashMap::new(),
            total: AtomicU64::new(0),
            max_recent,
        }
    }

    /// Record an error.
    pub fn record(
        &self,
        method: &str,
        path: &str,
        status_code: u16,
        error_message: &str,
        request_id: Option<&str>,
    ) {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.total.fetch_add(1, Ordering::Relaxed);

        // Store in recent buffer
        let record = ErrorRecord {
            timestamp,
            method: method.to_string(),
            path: path.to_string(),
            status_code,
            error_message: error_message.to_string(),
            request_id: request_id.map(|s| s.to_string()),
        };

        {
            let mut errors = self.errors.write().unwrap();
            if errors.len() >= self.max_recent {
                errors.remove(0);
            }
            errors.push(record);
        }

        // Update pattern
        let pattern_key = format!("{} {} {}", status_code, method, path);
        self.patterns
            .entry(pattern_key.clone())
            .and_modify(|p| {
                p.last_seen = timestamp;
                p.count += 1;
            })
            .or_insert(ErrorPattern {
                pattern_key,
                first_seen: timestamp,
                last_seen: timestamp,
                count: 1,
                sample_message: error_message.to_string(),
            });
    }

    /// Get recent errors.
    pub fn recent(&self, limit: usize) -> Vec<ErrorRecord> {
        let errors = self.errors.read().unwrap();
        let start = errors.len().saturating_sub(limit);
        errors[start..].to_vec()
    }

    /// Get error patterns sorted by count (descending).
    pub fn top_patterns(&self, limit: usize) -> Vec<ErrorPattern> {
        let mut patterns: Vec<ErrorPattern> =
            self.patterns.iter().map(|e| e.value().clone()).collect();
        patterns.sort_by(|a, b| b.count.cmp(&a.count));
        patterns.truncate(limit);
        patterns
    }

    /// Get total error count.
    pub fn error_count(&self) -> u64 {
        self.total.load(Ordering::Relaxed)
    }

    /// Get number of unique error patterns.
    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }

    /// Clear all tracked errors.
    pub fn clear(&self) {
        self.errors.write().unwrap().clear();
        self.patterns.clear();
        self.total.store(0, Ordering::Relaxed);
    }
}

impl Default for ErrorTracker {
    fn default() -> Self {
        Self::new(1000)
    }
}
