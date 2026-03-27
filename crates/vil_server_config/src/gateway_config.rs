// =============================================================================
// vil.yaml — Gateway/Pipeline Configuration
// =============================================================================

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Top-level vil.yaml for gateway/pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GatewayConfig {
    pub gateway: GatewaySection,
    pub runtime: RuntimeSection,
    pub logging: LoggingSection,
    pub grpc: GrpcSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GatewaySection {
    pub port: u16,
    pub host: String,
    pub path: String,
    pub upstream: UpstreamSection,
    pub post_body: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UpstreamSection {
    pub url: String,
    pub timeout_secs: u64,
    pub format: String,
    pub json_tap: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RuntimeSection {
    pub mode: String,
    pub shm: ShmBasicSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ShmBasicSection {
    pub enabled: bool,
    pub pool_size: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingSection {
    pub level: String,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GrpcSection {
    pub enabled: bool,
    pub port: u16,
}

// Defaults
impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            gateway: GatewaySection::default(),
            runtime: RuntimeSection::default(),
            logging: LoggingSection::default(),
            grpc: GrpcSection::default(),
        }
    }
}

impl Default for GatewaySection {
    fn default() -> Self {
        Self {
            port: 3080, host: "0.0.0.0".into(), path: "/trigger".into(),
            upstream: UpstreamSection::default(), post_body: None,
        }
    }
}

impl Default for UpstreamSection {
    fn default() -> Self {
        Self { url: String::new(), timeout_secs: 30, format: "sse".into(), json_tap: "choices[0].delta.content".into() }
    }
}

impl Default for RuntimeSection {
    fn default() -> Self {
        Self { mode: "shared".into(), shm: ShmBasicSection::default() }
    }
}

impl Default for ShmBasicSection {
    fn default() -> Self {
        Self { enabled: true, pool_size: "16MB".into() }
    }
}

impl Default for LoggingSection {
    fn default() -> Self {
        Self { level: "info".into(), format: "text".into() }
    }
}

impl Default for GrpcSection {
    fn default() -> Self {
        Self { enabled: false, port: 50051 }
    }
}

impl GatewayConfig {
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        serde_yaml::from_str(&content)
            .map_err(|e| format!("Failed to parse vil.yaml: {}", e))
    }

    pub fn from_str(yaml: &str) -> Result<Self, String> {
        serde_yaml::from_str(yaml).map_err(|e| format!("Parse error: {}", e))
    }
}
