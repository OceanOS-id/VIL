//! HTTP handlers for the A/B test plugin — wired to real ExperimentRegistry state.

use std::sync::Arc;
use tokio::sync::RwLock;
use vil_server::prelude::*;

use crate::experiment::{ExpStatus, Experiment};

/// Shared experiment registry holding all active experiments.
pub struct ExperimentRegistry {
    experiments: RwLock<Vec<Experiment>>,
}

impl ExperimentRegistry {
    pub fn new() -> Self {
        Self {
            experiments: RwLock::new(Vec::new()),
        }
    }

    /// Add an experiment to the registry.
    pub async fn add(&self, experiment: Experiment) {
        self.experiments.write().await.push(experiment);
    }

    /// List all experiments.
    pub async fn list(&self) -> Vec<Experiment> {
        self.experiments.read().await.clone()
    }

    /// Get an experiment by name.
    pub async fn get(&self, name: &str) -> Option<Experiment> {
        self.experiments
            .read()
            .await
            .iter()
            .find(|e| e.name == name)
            .cloned()
    }
}

impl Default for ExperimentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── Response types ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ExperimentSummary {
    pub name: String,
    pub variant_count: usize,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AbTestStatsBody {
    pub experiment_count: usize,
    pub experiments: Vec<ExperimentSummary>,
}

// ── Handlers ────────────────────────────────────────────────────────

/// GET /stats — return real experiment data from the registry.
pub async fn stats_handler(ctx: ServiceCtx) -> HandlerResult<VilResponse<AbTestStatsBody>> {
    let registry = ctx
        .state::<Arc<ExperimentRegistry>>()
        .expect("ExperimentRegistry");
    let experiments = registry.list().await;
    let summaries: Vec<ExperimentSummary> = experiments
        .iter()
        .map(|e| ExperimentSummary {
            name: e.name.clone(),
            variant_count: e.variants.len(),
            status: match e.status {
                ExpStatus::Draft => "draft".into(),
                ExpStatus::Running => "running".into(),
                ExpStatus::Paused => "paused".into(),
                ExpStatus::Completed => "completed".into(),
            },
        })
        .collect();

    Ok(VilResponse::ok(AbTestStatsBody {
        experiment_count: summaries.len(),
        experiments: summaries,
    }))
}
