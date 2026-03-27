//! VIL pattern HTTP handlers for the SQL agent plugin.

use vil_server::prelude::*;

use std::sync::Arc;

use crate::schema::SchemaRegistry;

// ── Request / Response types ────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct GenerateRequest {
    pub query: String,
}

#[derive(Debug, Serialize)]
pub struct GenerateResponseBody {
    pub sql: String,
    pub is_safe: bool,
    pub params: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SqlAgentStatsBody {
    pub table_count: usize,
    pub tables: Vec<String>,
    pub version: String,
}

// ── Handlers ────────────────────────────────────────────────────────

/// POST /generate — Generate SQL from natural language.
pub async fn generate_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<GenerateResponseBody>> {
    let registry = ctx.state::<Arc<SchemaRegistry>>()?;
    let req: GenerateRequest = body.json().map_err(|e| VilError::bad_request(e.to_string()))?;
    if req.query.trim().is_empty() {
        return Err(VilError::bad_request("query must not be empty"));
    }

    let schema_text = registry.to_schema_text();

    Ok(VilResponse::ok(GenerateResponseBody {
        sql: format!("-- schema context: {} tables\n-- query: {}", registry.table_names().len(), req.query),
        is_safe: true,
        params: vec![schema_text],
    }))
}

/// GET /stats — SQL agent service stats.
pub async fn stats_handler(
    ctx: ServiceCtx,
) -> VilResponse<SqlAgentStatsBody> {
    let registry = ctx.state::<Arc<SchemaRegistry>>().expect("SchemaRegistry");
    let tables: Vec<String> = registry.table_names().iter().map(|s| s.to_string()).collect();
    let table_count = tables.len();
    VilResponse::ok(SqlAgentStatsBody {
        table_count,
        tables,
        version: env!("CARGO_PKG_VERSION").into(),
    })
}
