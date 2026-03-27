// =============================================================================
// VIL Server — Alerting Rules Engine
// =============================================================================
//
// Threshold-based alerting for server metrics.
// Rules are evaluated periodically and trigger callbacks when conditions are met.
//
// Example rules:
//   - Alert if error rate > 5% for 60s
//   - Alert if p99 latency > 500ms
//   - Alert if SHM utilization > 80%
//   - Alert if in-flight requests > 1000

use serde::Serialize;
use std::time::{Duration, Instant};

/// Alert severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// Alert state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum AlertState {
    /// Normal — threshold not exceeded
    Ok,
    /// Threshold exceeded, within grace period
    Pending,
    /// Alert fired
    Firing,
    /// Was firing, now resolved
    Resolved,
}

/// An alert rule definition.
#[derive(Debug, Clone)]
pub struct AlertRule {
    /// Rule name (unique identifier)
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Severity level
    pub severity: AlertSeverity,
    /// Metric to evaluate
    pub metric: String,
    /// Threshold comparison
    pub condition: AlertCondition,
    /// How long the condition must be true before firing
    pub for_duration: Duration,
}

/// Threshold comparison condition.
#[derive(Debug, Clone)]
pub enum AlertCondition {
    GreaterThan(f64),
    LessThan(f64),
    EqualTo(f64),
}

impl AlertCondition {
    pub fn evaluate(&self, value: f64) -> bool {
        match self {
            Self::GreaterThan(threshold) => value > *threshold,
            Self::LessThan(threshold) => value < *threshold,
            Self::EqualTo(threshold) => (value - threshold).abs() < f64::EPSILON,
        }
    }

    pub fn description(&self) -> String {
        match self {
            Self::GreaterThan(t) => format!("> {}", t),
            Self::LessThan(t) => format!("< {}", t),
            Self::EqualTo(t) => format!("== {}", t),
        }
    }
}

/// Runtime state of an alert.
#[derive(Debug, Clone, Serialize)]
pub struct AlertStatus {
    pub name: String,
    pub description: String,
    pub severity: AlertSeverity,
    pub state: AlertState,
    pub current_value: f64,
    pub threshold: String,
    pub firing_since: Option<u64>, // unix timestamp
}

/// Alert engine — evaluates rules against current metrics.
pub struct AlertEngine {
    rules: Vec<AlertRule>,
    /// When each rule first exceeded threshold
    pending_since: std::collections::HashMap<String, Instant>,
    /// Currently firing alerts
    firing: std::collections::HashSet<String>,
}

impl AlertEngine {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            pending_since: std::collections::HashMap::new(),
            firing: std::collections::HashSet::new(),
        }
    }

    /// Add an alert rule.
    pub fn add_rule(&mut self, rule: AlertRule) {
        self.rules.push(rule);
    }

    /// Evaluate all rules against provided metric values.
    /// Returns list of alert statuses.
    pub fn evaluate(&mut self, metrics: &std::collections::HashMap<String, f64>) -> Vec<AlertStatus> {
        let mut statuses = Vec::new();

        for rule in &self.rules {
            let value = metrics.get(&rule.metric).copied().unwrap_or(0.0);
            let exceeded = rule.condition.evaluate(value);

            let state = if exceeded {
                if let Some(since) = self.pending_since.get(&rule.name) {
                    if since.elapsed() >= rule.for_duration {
                        self.firing.insert(rule.name.clone());
                        AlertState::Firing
                    } else {
                        AlertState::Pending
                    }
                } else {
                    self.pending_since.insert(rule.name.clone(), Instant::now());
                    AlertState::Pending
                }
            } else {
                self.pending_since.remove(&rule.name);
                if self.firing.remove(&rule.name) {
                    AlertState::Resolved
                } else {
                    AlertState::Ok
                }
            };

            statuses.push(AlertStatus {
                name: rule.name.clone(),
                description: rule.description.clone(),
                severity: rule.severity,
                state,
                current_value: value,
                threshold: rule.condition.description(),
                firing_since: if state == AlertState::Firing {
                    self.pending_since.get(&rule.name).map(|s| {
                        std::time::SystemTime::now()
                            .duration_since(std::time::SystemTime::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs()
                            .saturating_sub(s.elapsed().as_secs())
                    })
                } else {
                    None
                },
            });
        }

        statuses
    }

    /// Get number of currently firing alerts.
    pub fn firing_count(&self) -> usize {
        self.firing.len()
    }

    /// Get all rule names.
    pub fn rule_names(&self) -> Vec<String> {
        self.rules.iter().map(|r| r.name.clone()).collect()
    }
}

impl Default for AlertEngine {
    fn default() -> Self {
        Self::new()
    }
}
