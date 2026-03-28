// =============================================================================
// VIL REST Handlers — Feature Store
// =============================================================================

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use vil_server::prelude::*;

use crate::key::FeatureKey;
use crate::store::{FeatureStore, FeatureValue};

// ── Request / Response types ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct GetFeatureRequest {
    pub entity_id: String,
    pub feature_name: String,
}

#[derive(Debug, Serialize)]
pub struct GetFeatureResponse {
    pub key: String,
    pub value: Option<FeatureValue>,
}

#[derive(Debug, Deserialize)]
pub struct SetFeatureRequest {
    pub entity_id: String,
    pub feature_name: String,
    pub data: Vec<f32>,
    pub ttl_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct SetFeatureResponse {
    pub key: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct FeatureStatsResponse {
    pub entry_count: usize,
    pub has_default_ttl: bool,
    pub default_ttl_ms: Option<u64>,
}

// ── Handlers ────────────────────────────────────────────────────────────────

/// GET /api/features/get — retrieve a feature by entity_id + feature_name.
pub async fn handle_get_feature(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<GetFeatureResponse>> {
    let store = ctx.state::<Arc<FeatureStore>>()?;
    let req: GetFeatureRequest = body
        .json()
        .map_err(|e| VilError::bad_request(e.to_string()))?;
    let key = FeatureKey::new(&req.entity_id, &req.feature_name);
    let value = store.get(&key);
    let resp = GetFeatureResponse {
        key: key.to_compound(),
        value,
    };
    Ok(VilResponse::ok(resp))
}

/// POST /api/features/set — set a feature value.
pub async fn handle_set_feature(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<SetFeatureResponse>> {
    let store = ctx.state::<Arc<FeatureStore>>()?;
    let req: SetFeatureRequest = body
        .json()
        .map_err(|e| VilError::bad_request(e.to_string()))?;
    let key = FeatureKey::new(&req.entity_id, &req.feature_name);
    let value = FeatureValue {
        data: req.data,
        version: 0,
        created_at: 0,
        ttl_ms: req.ttl_ms,
    };
    store.set(&key, value);
    let resp = SetFeatureResponse {
        key: key.to_compound(),
        status: "ok".to_string(),
    };
    Ok(VilResponse::ok(resp))
}

/// GET /api/features/stats — return store statistics.
pub async fn handle_feature_stats(
    ctx: ServiceCtx,
) -> HandlerResult<VilResponse<FeatureStatsResponse>> {
    let store = ctx.state::<Arc<FeatureStore>>()?;
    let resp = FeatureStatsResponse {
        entry_count: store.count(),
        has_default_ttl: store.default_ttl_ms.is_some(),
        default_ttl_ms: store.default_ttl_ms,
    };
    Ok(VilResponse::ok(resp))
}
