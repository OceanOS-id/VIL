//! VIL pattern HTTP handlers for the GraphRAG plugin.

use vil_server::prelude::*;

use std::sync::Arc;

use vil_memory_graph::prelude::MemoryGraph;

use crate::query::GraphRagQuery;

// ── Request / Response types ────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct GraphRagQueryRequest {
    pub query: String,
    #[serde(default = "default_max_hops")]
    pub max_hops: usize,
    #[serde(default = "default_max_results")]
    pub max_results: usize,
}

fn default_max_hops() -> usize { 2 }
fn default_max_results() -> usize { 10 }

#[derive(Debug, Serialize)]
pub struct GraphRagQueryResponse {
    pub entities: Vec<EntitySummary>,
    pub relations: Vec<RelationSummary>,
    pub context: String,
}

#[derive(Debug, Serialize)]
pub struct EntitySummary {
    pub name: String,
    pub entity_type: String,
}

#[derive(Debug, Serialize)]
pub struct RelationSummary {
    pub from: String,
    pub to: String,
    pub relation_type: String,
    pub weight: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphRagStatsBody {
    pub entity_count: usize,
    pub version: String,
}

// ── Handlers ────────────────────────────────────────────────────────

/// POST /query — Execute a graph-enhanced RAG query.
pub async fn query_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<GraphRagQueryResponse>> {
    let graph = ctx.state::<Arc<MemoryGraph>>()?;
    let req: GraphRagQueryRequest = body.json().map_err(|e| VilError::bad_request(e.to_string()))?;
    if req.query.trim().is_empty() {
        return Err(VilError::bad_request("query must not be empty"));
    }

    let result = GraphRagQuery::new(&graph)
        .max_hops(req.max_hops)
        .max_results(req.max_results)
        .query(&req.query);

    let entities: Vec<EntitySummary> = result
        .entities
        .iter()
        .map(|e| EntitySummary {
            name: e.name.clone(),
            entity_type: e.entity_type.clone(),
        })
        .collect();

    let relations: Vec<RelationSummary> = result
        .relations
        .iter()
        .map(|r| RelationSummary {
            from: r.from_name.clone(),
            to: r.to_name.clone(),
            relation_type: r.relation_type.clone(),
            weight: r.weight,
        })
        .collect();

    Ok(VilResponse::ok(GraphRagQueryResponse {
        entities,
        relations,
        context: result.context,
    }))
}

/// GET /stats — GraphRAG service stats.
pub async fn stats_handler(
    ctx: ServiceCtx,
) -> VilResponse<GraphRagStatsBody> {
    let graph = ctx.state::<Arc<MemoryGraph>>().expect("MemoryGraph");
    VilResponse::ok(GraphRagStatsBody {
        entity_count: graph.entity_count(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
}
