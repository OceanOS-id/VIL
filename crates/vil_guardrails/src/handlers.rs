use vil_server::prelude::*;

use std::sync::Arc;
use crate::{GuardrailsEngine, GuardrailResult};

#[derive(Debug, Deserialize)]
pub struct ValidateRequest {
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct ValidateResponseBody {
    pub result: GuardrailResult,
}

#[derive(Debug, Serialize)]
pub struct GuardrailStatsBody {
    pub checks_available: Vec<String>,
    pub version: String,
}

pub async fn validate_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<ValidateResponseBody>> {
    let engine = ctx.state::<Arc<GuardrailsEngine>>().expect("GuardrailsEngine");
    let req: ValidateRequest = body.json().expect("invalid JSON");
    if req.text.is_empty() {
        return Err(VilError::bad_request("text must not be empty"));
    }
    let result = engine.check(&req.text);
    Ok(VilResponse::ok(ValidateResponseBody { result }))
}

pub async fn stats_handler() -> HandlerResult<VilResponse<GuardrailStatsBody>> {
    Ok(VilResponse::ok(GuardrailStatsBody {
        checks_available: vec![
            "pii_detection".into(),
            "toxicity_scoring".into(),
            "custom_rules".into(),
        ],
        version: env!("CARGO_PKG_VERSION").into(),
    }))
}
