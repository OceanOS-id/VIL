//! Privacy audit log — track what was redacted/anonymized.

use serde::{Deserialize, Serialize};

/// A single audit entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp_ms: u64,
    pub operation: AuditOperation,
    pub pattern_name: Option<String>,
    pub items_affected: usize,
    pub document_id: Option<String>,
}

/// Type of privacy operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditOperation {
    Redaction,
    Anonymization,
}

/// Privacy audit log.
#[derive(Debug, Clone, Default)]
pub struct PrivacyAuditLog {
    pub entries: Vec<AuditEntry>,
}

impl PrivacyAuditLog {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Log a redaction event.
    pub fn log_redaction(
        &mut self,
        pattern_name: &str,
        items_affected: usize,
        document_id: Option<&str>,
    ) {
        self.entries.push(AuditEntry {
            timestamp_ms: current_time_ms(),
            operation: AuditOperation::Redaction,
            pattern_name: Some(pattern_name.to_string()),
            items_affected,
            document_id: document_id.map(|s| s.to_string()),
        });
    }

    /// Log an anonymization event.
    pub fn log_anonymization(&mut self, items_affected: usize, document_id: Option<&str>) {
        self.entries.push(AuditEntry {
            timestamp_ms: current_time_ms(),
            operation: AuditOperation::Anonymization,
            pattern_name: None,
            items_affected,
            document_id: document_id.map(|s| s.to_string()),
        });
    }

    /// Total number of audit entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Export audit log as JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&self.entries).unwrap_or_default()
    }
}

fn current_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
