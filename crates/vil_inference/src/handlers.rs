use crate::registry::ModelRegistry;
use serde::Serialize;
use std::sync::Arc;
use vil_server::prelude::*;

#[derive(Debug, Serialize)]
pub struct ModelsResponse {
    pub models: Vec<String>,
    pub count: usize,
}

pub async fn list_models_handler(ctx: ServiceCtx) -> VilResponse<ModelsResponse> {
    let registry = ctx.state::<Arc<ModelRegistry>>().expect("ModelRegistry");
    let models = registry.list();
    let count = models.len();
    VilResponse::ok(ModelsResponse { models, count })
}
