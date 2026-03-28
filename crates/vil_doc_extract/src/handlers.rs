//! VIL pattern HTTP handlers for the document extraction plugin.

use vil_server::prelude::*;

use std::collections::HashMap;
use std::sync::Arc;

use crate::extractor::DataExtractor;
use crate::field::FieldDef;
use crate::rules::{invoice_fields, receipt_fields, resume_fields};

// ── Request / Response types ────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ExtractRequest {
    pub text: String,
    #[serde(default)]
    pub template: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExtractResponseBody {
    pub fields: HashMap<String, ExtractedFieldSummary>,
    pub confidence: f32,
    pub is_complete: bool,
    pub missing_required: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ExtractedFieldSummary {
    pub value: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExtractStatsBody {
    pub available_templates: Vec<String>,
    pub version: String,
}

// ── Handlers ────────────────────────────────────────────────────────

/// POST /extract — Extract structured data from text.
pub async fn extract_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<ExtractResponseBody>> {
    let extractor = ctx
        .state::<Arc<dyn DataExtractor>>()
        .expect("DataExtractor");
    let req: ExtractRequest = body.json().expect("invalid JSON");
    if req.text.trim().is_empty() {
        return Err(VilError::bad_request("text must not be empty"));
    }

    let fields: Vec<FieldDef> = match req.template.as_deref() {
        Some("invoice") => invoice_fields(),
        Some("receipt") => receipt_fields(),
        Some("resume") => resume_fields(),
        Some(other) => {
            return Err(VilError::bad_request(format!(
                "unknown template: {}",
                other
            )))
        }
        None => invoice_fields(),
    };

    let result = extractor.extract(&req.text, &fields);

    let field_summaries: HashMap<String, ExtractedFieldSummary> = result
        .fields
        .into_iter()
        .map(|(k, v)| {
            (
                k,
                ExtractedFieldSummary {
                    value: v.value,
                    confidence: v.confidence,
                },
            )
        })
        .collect();

    let is_complete = result.missing_required.is_empty();

    Ok(VilResponse::ok(ExtractResponseBody {
        fields: field_summaries,
        confidence: result.confidence,
        is_complete,
        missing_required: result.missing_required,
    }))
}

/// GET /stats — Extraction service stats.
pub async fn stats_handler() -> VilResponse<ExtractStatsBody> {
    VilResponse::ok(ExtractStatsBody {
        available_templates: vec!["invoice".into(), "receipt".into(), "resume".into()],
        version: env!("CARGO_PKG_VERSION").into(),
    })
}
