//! VIL pattern HTTP handlers for the model registry plugin.

use vil_server::prelude::*;

use std::sync::Arc;

use crate::model::ModelStatus;
use crate::registry::ModelRegistry;

// ── Response types ──────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ModelsResponseBody {
    pub models: Vec<ModelSummary>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct ModelSummary {
    pub name: String,
    pub version: u32,
    pub provider: String,
    pub status: String,
}

impl ModelSummary {
    pub fn from_entry(entry: &crate::model::ModelEntry) -> Self {
        Self {
            name: entry.name.clone(),
            version: entry.version,
            provider: entry.provider.clone(),
            status: match entry.status {
                ModelStatus::Staging => "staging".into(),
                ModelStatus::Active => "active".into(),
                ModelStatus::Deprecated => "deprecated".into(),
                ModelStatus::Archived => "archived".into(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RegistryStatsBody {
    pub model_count: usize,
    pub version: String,
}

// ── Handlers ────────────────────────────────────────────────────────

/// GET /models — List all registered models.
pub async fn models_handler(
    ctx: ServiceCtx,
) -> VilResponse<ModelsResponseBody> {
    let registry = ctx.state::<Arc<ModelRegistry>>().expect("ModelRegistry");
    let all = registry.list();
    let models: Vec<ModelSummary> = all
        .iter()
        .flat_map(|(_, entries)| entries.iter().map(ModelSummary::from_entry))
        .collect();
    let total = models.len();
    VilResponse::ok(ModelsResponseBody { models, total })
}

/// GET /stats — Registry service stats.
pub async fn stats_handler(
    ctx: ServiceCtx,
) -> VilResponse<RegistryStatsBody> {
    let registry = ctx.state::<Arc<ModelRegistry>>().expect("ModelRegistry");
    VilResponse::ok(RegistryStatsBody {
        model_count: registry.list().len(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
}
