//! HTTP handlers for the AI compiler plugin — wired to real PipelineDag state.

use std::sync::Arc;
use tokio::sync::RwLock;
use vil_server::prelude::*;

use crate::compiler::{compile, CompiledPlan};
use crate::dag::PipelineDag;

/// Shared compiler state holding the current DAG and its compiled plan.
pub struct CompilerStats {
    dag: RwLock<Option<PipelineDag>>,
    plan: RwLock<Option<CompiledPlan>>,
}

impl CompilerStats {
    pub fn new() -> Self {
        Self {
            dag: RwLock::new(None),
            plan: RwLock::new(None),
        }
    }

    /// Load a DAG and compile it, storing both.
    pub async fn load_dag(&self, dag: PipelineDag) -> Result<(), String> {
        let plan = compile(&dag).map_err(|e| e.to_string())?;
        *self.dag.write().await = Some(dag);
        *self.plan.write().await = Some(plan);
        Ok(())
    }
}

impl Default for CompilerStats {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Serialize)]
pub struct StatsResponseBody {
    pub has_dag: bool,
    pub node_count: usize,
    pub edge_count: usize,
    pub compiled_steps: usize,
    pub parallel_tiers: usize,
    pub supported_nodes: Vec<String>,
    pub optimization_passes: Vec<String>,
}

pub async fn stats_handler(ctx: ServiceCtx) -> HandlerResult<VilResponse<StatsResponseBody>> {
    let state = ctx.state::<Arc<CompilerStats>>().expect("CompilerStats");
    let dag = state.dag.read().await;
    let plan = state.plan.read().await;

    let (has_dag, node_count, edge_count) = match dag.as_ref() {
        Some(d) => (true, d.node_count(), d.edge_count()),
        None => (false, 0, 0),
    };

    let (compiled_steps, parallel_tiers) = match plan.as_ref() {
        Some(p) => (p.step_count(), p.tier_count()),
        None => (0, 0),
    };

    Ok(VilResponse::ok(StatsResponseBody {
        has_dag,
        node_count,
        edge_count,
        compiled_steps,
        parallel_tiers,
        supported_nodes: vec![
            "Embed".into(),
            "Search".into(),
            "Rerank".into(),
            "Generate".into(),
            "Transform".into(),
            "Cache".into(),
            "Filter".into(),
            "Merge".into(),
        ],
        optimization_passes: vec![
            "transform_fusion".into(),
            "redundant_cache_elimination".into(),
            "parallel_tier_grouping".into(),
        ],
    }))
}
