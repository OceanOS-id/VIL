use dashmap::DashMap;
use std::time::Instant;
use serde::{Serialize, Deserialize};
use std::collections::VecDeque;
use vil_macros::VilAiState;

/// Rolling window size for error rate and latency calculations.
const WINDOW_SIZE: usize = 100;

/// Health status of a model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Model is operating normally (error_rate <= 10%).
    Healthy,
    /// Model has elevated error rate (10% < error_rate <= 50%).
    Degraded,
    /// Model is failing (error_rate > 50% or circuit open).
    Unhealthy,
    /// No data yet.
    Unknown,
}

/// Snapshot of a model's health.
#[derive(Debug, Clone, Serialize, VilAiState)]
pub struct ModelHealth {
    pub model: String,
    pub status: HealthStatus,
    pub total_requests: u64,
    pub total_errors: u64,
    pub error_rate: f64,
    pub avg_latency_ms: f64,
    pub p99_latency_ms: u64,
    #[serde(skip)]
    pub last_success: Option<Instant>,
    #[serde(skip)]
    pub last_failure: Option<Instant>,
}

/// Internal mutable state for a model.
pub(crate) struct ModelHealthState {
    pub total_requests: u64,
    pub total_errors: u64,
    /// Rolling window of (is_error, latency_ms).
    pub window: VecDeque<(bool, u64)>,
    pub last_success: Option<Instant>,
    pub last_failure: Option<Instant>,
}

impl ModelHealthState {
    fn new() -> Self {
        Self {
            total_requests: 0,
            total_errors: 0,
            window: VecDeque::with_capacity(WINDOW_SIZE),
            last_success: None,
            last_failure: None,
        }
    }

    fn error_rate(&self) -> f64 {
        if self.window.is_empty() {
            return 0.0;
        }
        let errors = self.window.iter().filter(|(e, _)| *e).count();
        errors as f64 / self.window.len() as f64
    }

    fn avg_latency_ms(&self) -> f64 {
        let latencies: Vec<u64> = self.window.iter().filter(|(e, _)| !*e).map(|(_, l)| *l).collect();
        if latencies.is_empty() {
            return 0.0;
        }
        latencies.iter().sum::<u64>() as f64 / latencies.len() as f64
    }

    fn p99_latency_ms(&self) -> u64 {
        let mut latencies: Vec<u64> = self.window.iter().filter(|(e, _)| !*e).map(|(_, l)| *l).collect();
        if latencies.is_empty() {
            return 0;
        }
        latencies.sort_unstable();
        let idx = ((latencies.len() as f64) * 0.99).ceil() as usize;
        latencies[idx.min(latencies.len()) - 1]
    }

    fn status(&self) -> HealthStatus {
        if self.total_requests == 0 {
            return HealthStatus::Unknown;
        }
        let rate = self.error_rate();
        if rate > 0.5 {
            HealthStatus::Unhealthy
        } else if rate > 0.1 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        }
    }

    fn to_health(&self, model: &str) -> ModelHealth {
        ModelHealth {
            model: model.to_string(),
            status: self.status(),
            total_requests: self.total_requests,
            total_errors: self.total_errors,
            error_rate: self.error_rate(),
            avg_latency_ms: self.avg_latency_ms(),
            p99_latency_ms: self.p99_latency_ms(),
            last_success: self.last_success,
            last_failure: self.last_failure,
        }
    }
}

/// Per-model health tracker with rolling window metrics.
pub struct HealthTracker {
    models: DashMap<String, ModelHealthState>,
}

impl HealthTracker {
    pub fn new() -> Self {
        Self {
            models: DashMap::new(),
        }
    }

    /// Record a successful request for the given model.
    pub fn record_success(&self, model: &str, latency_ms: u64) {
        let mut entry = self.models.entry(model.to_string()).or_insert_with(ModelHealthState::new);
        let state = entry.value_mut();
        state.total_requests += 1;
        state.last_success = Some(Instant::now());
        if state.window.len() >= WINDOW_SIZE {
            state.window.pop_front();
        }
        state.window.push_back((false, latency_ms));
    }

    /// Record a failed request for the given model.
    pub fn record_failure(&self, model: &str, _error: &str) {
        let mut entry = self.models.entry(model.to_string()).or_insert_with(ModelHealthState::new);
        let state = entry.value_mut();
        state.total_requests += 1;
        state.total_errors += 1;
        state.last_failure = Some(Instant::now());
        if state.window.len() >= WINDOW_SIZE {
            state.window.pop_front();
        }
        state.window.push_back((true, 0));
    }

    /// Get health snapshot for a specific model.
    pub fn get_health(&self, model: &str) -> ModelHealth {
        match self.models.get(model) {
            Some(entry) => entry.value().to_health(model),
            None => ModelHealth {
                model: model.to_string(),
                status: HealthStatus::Unknown,
                total_requests: 0,
                total_errors: 0,
                error_rate: 0.0,
                avg_latency_ms: 0.0,
                p99_latency_ms: 0,
                last_success: None,
                last_failure: None,
            },
        }
    }

    /// Get health snapshots for all tracked models.
    pub fn get_all(&self) -> Vec<ModelHealth> {
        self.models
            .iter()
            .map(|entry| entry.value().to_health(entry.key()))
            .collect()
    }

    /// Return the healthiest model with the lowest average latency.
    pub fn best_model(&self) -> Option<String> {
        let mut best: Option<(String, f64)> = None;
        for entry in self.models.iter() {
            let state = entry.value();
            let status = state.status();
            if status == HealthStatus::Unhealthy {
                continue;
            }
            let latency = state.avg_latency_ms();
            match &best {
                None => best = Some((entry.key().clone(), latency)),
                Some((_, best_lat)) => {
                    if latency < *best_lat {
                        best = Some((entry.key().clone(), latency));
                    }
                }
            }
        }
        best.map(|(m, _)| m)
    }

    /// Check if a model is healthy (Healthy or Unknown status).
    pub fn is_healthy(&self, model: &str) -> bool {
        match self.models.get(model) {
            Some(entry) => {
                let status = entry.value().status();
                status == HealthStatus::Healthy || status == HealthStatus::Unknown
            }
            None => true, // unknown = allow
        }
    }
}

impl Default for HealthTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_success_and_health() {
        let tracker = HealthTracker::new();
        tracker.record_success("gpt-4", 100);
        tracker.record_success("gpt-4", 200);

        let health = tracker.get_health("gpt-4");
        assert_eq!(health.status, HealthStatus::Healthy);
        assert_eq!(health.total_requests, 2);
        assert_eq!(health.total_errors, 0);
        assert!((health.avg_latency_ms - 150.0).abs() < 0.01);
    }

    #[test]
    fn test_record_failure_degrades_health() {
        let tracker = HealthTracker::new();
        // 8 successes + 2 failures = 20% error rate → Degraded
        for _ in 0..8 {
            tracker.record_success("gpt-4", 100);
        }
        for _ in 0..2 {
            tracker.record_failure("gpt-4", "timeout");
        }
        let health = tracker.get_health("gpt-4");
        assert_eq!(health.status, HealthStatus::Degraded);
        assert_eq!(health.total_errors, 2);
    }

    #[test]
    fn test_unhealthy_on_high_error_rate() {
        let tracker = HealthTracker::new();
        // 6 failures out of 10 = 60% → Unhealthy
        for _ in 0..4 {
            tracker.record_success("claude", 50);
        }
        for _ in 0..6 {
            tracker.record_failure("claude", "error");
        }
        let health = tracker.get_health("claude");
        assert_eq!(health.status, HealthStatus::Unhealthy);
    }

    #[test]
    fn test_unknown_model() {
        let tracker = HealthTracker::new();
        let health = tracker.get_health("nonexistent");
        assert_eq!(health.status, HealthStatus::Unknown);
    }

    #[test]
    fn test_best_model() {
        let tracker = HealthTracker::new();
        tracker.record_success("fast", 50);
        tracker.record_success("slow", 200);
        assert_eq!(tracker.best_model(), Some("fast".to_string()));
    }

    #[test]
    fn test_best_model_skips_unhealthy() {
        let tracker = HealthTracker::new();
        // Make "bad" unhealthy
        for _ in 0..10 {
            tracker.record_failure("bad", "err");
        }
        tracker.record_success("good", 100);
        assert_eq!(tracker.best_model(), Some("good".to_string()));
    }

    #[test]
    fn test_get_all() {
        let tracker = HealthTracker::new();
        tracker.record_success("a", 10);
        tracker.record_success("b", 20);
        let all = tracker.get_all();
        assert_eq!(all.len(), 2);
    }
}
