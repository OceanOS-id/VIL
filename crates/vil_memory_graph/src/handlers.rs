use crate::graph::MemoryGraph;
use std::sync::Arc;
use vil_server::prelude::*;

#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    pub query: String,
    pub top_k: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct EntityHit {
    pub id: u64,
    pub name: String,
    pub entity_type: String,
}

#[derive(Debug, Serialize)]
pub struct QueryResponseBody {
    pub results: Vec<EntityHit>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct StatsResponseBody {
    pub entity_count: usize,
    pub relation_count: usize,
}

pub async fn query_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<QueryResponseBody>> {
    let graph = ctx.state::<Arc<MemoryGraph>>()?;
    let req: QueryRequest = body
        .json()
        .map_err(|e| VilError::bad_request(e.to_string()))?;
    let top_k = req.top_k.unwrap_or(10);
    let results = graph.recall(&req.query, top_k);
    let hits: Vec<EntityHit> = results
        .iter()
        .map(|e| EntityHit {
            id: e.id,
            name: e.name.clone(),
            entity_type: format!("{:?}", e.entity_type),
        })
        .collect();
    let count = hits.len();
    Ok(VilResponse::ok(QueryResponseBody {
        results: hits,
        count,
    }))
}

pub async fn stats_handler(ctx: ServiceCtx) -> VilResponse<StatsResponseBody> {
    let graph = ctx.state::<Arc<MemoryGraph>>().expect("MemoryGraph");
    VilResponse::ok(StatsResponseBody {
        entity_count: graph.entity_count(),
        relation_count: graph.relation_count(),
    })
}
