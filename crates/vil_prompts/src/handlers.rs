use vil_server::prelude::*;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::PromptRegistry;

#[derive(Debug, Deserialize)]
pub struct RenderRequest {
    pub template_name: String,
    pub variables: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct RenderResponseBody {
    pub rendered: String,
}

#[derive(Debug, Serialize)]
pub struct PromptListEntry {
    pub name: String,
    pub variable_count: usize,
}

#[derive(Debug, Serialize)]
pub struct PromptListResponseBody {
    pub templates: Vec<PromptListEntry>,
    pub count: usize,
}

pub async fn render_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<RenderResponseBody>> {
    let registry = ctx.state::<Arc<RwLock<PromptRegistry>>>().expect("PromptRegistry");
    let req: RenderRequest = body.json().expect("invalid JSON");
    let reg = registry.read().map_err(|_| VilError::internal("lock poisoned"))?;
    let rendered = reg.render(&req.template_name, &req.variables)
        .ok_or_else(|| VilError::not_found("template not found"))?
        .map_err(|e| VilError::bad_request(e.to_string()))?;
    Ok(VilResponse::ok(RenderResponseBody { rendered }))
}

pub async fn list_handler(
    ctx: ServiceCtx,
) -> HandlerResult<VilResponse<PromptListResponseBody>> {
    let registry = ctx.state::<Arc<RwLock<PromptRegistry>>>().expect("PromptRegistry");
    let reg = registry.read().map_err(|_| VilError::internal("lock poisoned"))?;
    let templates: Vec<PromptListEntry> = reg.names()
        .map(|name| {
            let var_count = reg.get(name).map(|t| t.variable_count()).unwrap_or(0);
            PromptListEntry { name: name.clone(), variable_count: var_count }
        })
        .collect();
    let count = templates.len();
    Ok(VilResponse::ok(PromptListResponseBody { templates, count }))
}
