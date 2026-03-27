//! VIL pattern HTTP handlers for the audio plugin.

use vil_server::prelude::*;

use std::sync::Arc;

use crate::transcriber::Transcriber;

// ── Request / Response types ────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct TranscribeRequest {
    /// Base64-encoded audio data.
    pub audio_base64: String,
    #[serde(default = "default_language")]
    pub language: String,
}

fn default_language() -> String { "auto".into() }

#[derive(Debug, Serialize)]
pub struct TranscribeResponseBody {
    pub text: String,
    pub language: String,
    pub backend: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AudioStatsBody {
    pub backend: String,
    pub version: String,
}

// ── Handlers ────────────────────────────────────────────────────────

/// POST /transcribe — Transcribe audio to text.
pub async fn transcribe_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<TranscribeResponseBody>> {
    let transcriber = ctx.state::<Arc<dyn Transcriber>>().expect("Transcriber");
    let req: TranscribeRequest = body.json().expect("invalid JSON");
    if req.audio_base64.trim().is_empty() {
        return Err(VilError::bad_request("audio_base64 must not be empty"));
    }

    let audio_bytes = req.audio_base64.as_bytes();

    match transcriber.transcribe(audio_bytes).await {
        Ok(transcript) => Ok(VilResponse::ok(TranscribeResponseBody {
            text: transcript.text,
            language: req.language,
            backend: transcriber.name().to_string(),
        })),
        Err(e) => Err(VilError::internal(format!("transcription failed: {}", e))),
    }
}

/// GET /stats — Audio service stats.
pub async fn stats_handler(
    ctx: ServiceCtx,
) -> VilResponse<AudioStatsBody> {
    let transcriber = ctx.state::<Arc<dyn Transcriber>>().expect("Transcriber");
    VilResponse::ok(AudioStatsBody {
        backend: transcriber.name().to_string(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
}
