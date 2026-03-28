//! VilPlugin implementation for audio transcription integration.

use vil_server::prelude::*;

use std::sync::Arc;

use crate::handlers;
use crate::semantic::{AudioEvent, AudioFault, AudioState};
use crate::transcriber::{NoopTranscriber, Transcriber};

/// Audio plugin — speech-to-text and text-to-speech.
pub struct AudioPlugin {
    transcriber: Arc<dyn Transcriber>,
}

impl AudioPlugin {
    pub fn new(transcriber: Arc<dyn Transcriber>) -> Self {
        Self { transcriber }
    }
}

impl Default for AudioPlugin {
    fn default() -> Self {
        Self {
            transcriber: Arc::new(NoopTranscriber),
        }
    }
}

impl VilPlugin for AudioPlugin {
    fn id(&self) -> &str {
        "vil-audio"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn description(&self) -> &str {
        "Audio transcription and speech processing"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "audio".into(),
            endpoints: vec![
                EndpointSpec::post("/api/audio/transcribe")
                    .with_description("Transcribe audio to text"),
                EndpointSpec::get("/api/audio/stats").with_description("Audio service stats"),
            ],
        }]
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        Vec::new()
    }

    fn register(&self, ctx: &mut PluginContext) {
        let svc = ServiceProcess::new("audio")
            .state(Arc::clone(&self.transcriber))
            .endpoint(
                Method::POST,
                "/transcribe",
                post(handlers::transcribe_handler),
            )
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .emits::<AudioEvent>()
            .faults::<AudioFault>()
            .manages::<AudioState>();

        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
