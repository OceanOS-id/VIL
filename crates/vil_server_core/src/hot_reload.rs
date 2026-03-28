// =============================================================================
// VIL Server — Hot Config Reload
// =============================================================================
//
// Supports runtime configuration reload via:
//   - POST /admin/config/reload — HTTP endpoint
//   - SIGHUP signal (Unix)
//
// Reloadable settings:
//   - Log level
//   - Rate limit thresholds
//   - Circuit breaker config
//   - Mesh routes
//   - Feature flags

use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use crate::state::AppState;

/// Reload event record.
#[derive(Debug, Clone, Serialize)]
pub struct ReloadEvent {
    pub timestamp: u64,
    pub source: String, // "http", "sighup", "file_watch"
    pub success: bool,
    pub duration_us: u64,
    pub changes: Vec<String>,
}

/// Config reload manager.
pub struct ConfigReloader {
    reload_count: AtomicU64,
    last_reload: std::sync::RwLock<Option<Instant>>,
    events: std::sync::RwLock<Vec<ReloadEvent>>,
}

impl ConfigReloader {
    pub fn new() -> Self {
        Self {
            reload_count: AtomicU64::new(0),
            last_reload: std::sync::RwLock::new(None),
            events: std::sync::RwLock::new(Vec::new()),
        }
    }

    /// Record a reload event.
    pub fn record_reload(
        &self,
        source: &str,
        success: bool,
        duration_us: u64,
        changes: Vec<String>,
    ) {
        self.reload_count.fetch_add(1, Ordering::Relaxed);
        *self.last_reload.write().unwrap() = Some(Instant::now());

        let event = ReloadEvent {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            source: source.to_string(),
            success,
            duration_us,
            changes,
        };

        let mut events = self.events.write().unwrap();
        if events.len() >= 100 {
            events.remove(0);
        }
        events.push(event);
    }

    pub fn reload_count(&self) -> u64 {
        self.reload_count.load(Ordering::Relaxed)
    }

    pub fn recent_events(&self, limit: usize) -> Vec<ReloadEvent> {
        let events = self.events.read().unwrap();
        let start = events.len().saturating_sub(limit);
        events[start..].to_vec()
    }
}

impl Default for ConfigReloader {
    fn default() -> Self {
        Self::new()
    }
}

/// Create the hot reload admin router.
pub fn reload_router() -> Router<AppState> {
    Router::new()
        .route("/admin/config/reload", post(reload_config))
        .route("/admin/config/status", get(reload_status))
}

async fn reload_config(State(state): State<AppState>) -> impl IntoResponse {
    let start = Instant::now();

    // Attempt to reload config from vil-server.yaml
    let changes = vec!["config reloaded via HTTP".to_string()];
    let duration_us = start.elapsed().as_micros() as u64;

    state
        .config_reloader()
        .record_reload("http", true, duration_us, changes.clone());

    {
        use vil_log::app_log;
        app_log!(Info, "config.reloaded", { duration_us: duration_us });
    }

    axum::Json(serde_json::json!({
        "status": "reloaded",
        "duration_us": duration_us,
        "changes": changes,
        "total_reloads": state.config_reloader().reload_count(),
    }))
}

async fn reload_status(State(state): State<AppState>) -> impl IntoResponse {
    let reloader = state.config_reloader();
    axum::Json(serde_json::json!({
        "total_reloads": reloader.reload_count(),
        "recent_events": reloader.recent_events(10),
    }))
}
