//! VIL pattern HTTP handlers for the vision plugin.

use vil_server::prelude::*;

use std::sync::Arc;

use crate::analyzer::ImageAnalyzer;
use crate::config::VisionConfig;

// ── Request / Response types ────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AnalyzeRequest {
    /// Base64-encoded image data.
    pub image_base64: String,
    #[serde(default = "default_ocr")]
    pub ocr_enabled: bool,
}

fn default_ocr() -> bool { true }

#[derive(Debug, Serialize)]
pub struct AnalyzeResponseBody {
    pub description: String,
    pub objects: Vec<DetectedObjectSummary>,
    pub text_content: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DetectedObjectSummary {
    pub label: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct VisionStatsBody {
    pub backend: String,
    pub config: VisionConfigSummary,
    pub version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VisionConfigSummary {
    pub max_dimension: u32,
    pub ocr_enabled: bool,
    pub object_detection: bool,
    pub min_confidence: f32,
    pub model: String,
}

// ── Handlers ────────────────────────────────────────────────────────

/// Newtype wrapper for dyn trait state (enables Any downcast via ServiceCtx).
#[derive(Clone)]
pub struct VisionAnalyzer(pub Arc<dyn ImageAnalyzer>);

/// POST /analyze — Analyze an image.
pub async fn analyze_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<AnalyzeResponseBody>> {
    let analyzer = &ctx.state::<VisionAnalyzer>().expect("VisionAnalyzer").0;
    let req: AnalyzeRequest = body.json().expect("invalid JSON");
    if req.image_base64.trim().is_empty() {
        return Err(VilError::bad_request("image_base64 must not be empty"));
    }

    let image_bytes = req.image_base64.as_bytes();

    match analyzer.analyze(image_bytes).await {
        Ok(analysis) => {
            let objects: Vec<DetectedObjectSummary> = analysis
                .objects
                .iter()
                .map(|o| DetectedObjectSummary {
                    label: o.label.clone(),
                    confidence: o.confidence,
                })
                .collect();

            Ok(VilResponse::ok(AnalyzeResponseBody {
                description: analysis.description,
                objects,
                text_content: analysis.text_content,
            }))
        }
        Err(e) => Err(VilError::internal(format!("analysis failed: {}", e))),
    }
}

/// GET /stats — Vision service stats.
pub async fn stats_handler(
    ctx: ServiceCtx,
) -> VilResponse<VisionStatsBody> {
    let analyzer = &ctx.state::<VisionAnalyzer>().expect("VisionAnalyzer").0;
    let config = ctx.state::<Arc<VisionConfig>>().expect("VisionConfig");
    VilResponse::ok(VisionStatsBody {
        backend: analyzer.name().to_string(),
        config: VisionConfigSummary {
            max_dimension: config.max_dimension,
            ocr_enabled: config.ocr_enabled,
            object_detection: config.object_detection,
            min_confidence: config.min_confidence,
            model: config.model.clone(),
        },
        version: env!("CARGO_PKG_VERSION").into(),
    })
}
