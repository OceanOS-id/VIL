// =============================================================================
// VIL Server Mesh — YAML Service Definition Loader
// =============================================================================
//
// Loads vil-server.yaml service definitions including:
//   - Service declarations with visibility and prefix
//   - Mesh route definitions (from/to/lane)
//   - Deployment mode (unified vs standalone)
//
// Example YAML:
//   server:
//     name: my-platform
//     port: 8080
//     metrics_port: 9090
//   services:
//     - name: auth
//       visibility: public
//       prefix: /auth
//     - name: orders
//       visibility: public
//       prefix: /api
//   mesh:
//     mode: unified
//     routes:
//       - from: auth
//         to: orders
//         lane: trigger

use serde::Deserialize;
use std::path::Path;

use crate::{Lane, MeshConfig, MeshMode, MeshRoute};

/// Top-level vil-server.yaml configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct VilServerYaml {
    #[serde(default)]
    pub server: ServerSection,
    #[serde(default)]
    pub services: Vec<ServiceEntry>,
    #[serde(default)]
    pub mesh: MeshSection,
    #[serde(default)]
    pub profiles: std::collections::HashMap<String, ProfileOverride>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerSection {
    pub name: String,
    pub port: u16,
    pub host: String,
    pub metrics_port: Option<u16>,
    pub workers: Option<usize>,
    pub request_timeout_secs: u64,
}

impl Default for ServerSection {
    fn default() -> Self {
        Self {
            name: "vil-server".to_string(),
            port: 8080,
            host: "0.0.0.0".to_string(),
            metrics_port: None,
            workers: None,
            request_timeout_secs: 30,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceEntry {
    pub name: String,
    #[serde(default = "default_visibility")]
    pub visibility: String,
    #[serde(default)]
    pub prefix: Option<String>,
}

fn default_visibility() -> String {
    "public".to_string()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct MeshSection {
    pub mode: DeploymentMode,
    pub routes: Vec<MeshRouteYaml>,
}

impl Default for MeshSection {
    fn default() -> Self {
        Self {
            mode: DeploymentMode::Unified,
            routes: Vec::new(),
        }
    }
}

/// Deployment mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeploymentMode {
    /// All services in one binary (default) — uses SHM for IPC
    Unified,
    /// Each service is a standalone binary — uses TCP
    Standalone,
}

impl Default for DeploymentMode {
    fn default() -> Self {
        Self::Unified
    }
}

/// Route definition in YAML (simplified).
#[derive(Debug, Clone, Deserialize)]
pub struct MeshRouteYaml {
    pub from: String,
    pub to: String,
    #[serde(default = "default_lane")]
    pub lane: String,
}

fn default_lane() -> String {
    "data".to_string()
}

/// Profile overrides for dev/staging/prod.
#[derive(Debug, Clone, Deserialize)]
pub struct ProfileOverride {
    pub log_level: Option<String>,
    pub workers: Option<usize>,
    pub port: Option<u16>,
}

impl VilServerYaml {
    /// Load from a YAML file path.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, YamlConfigError> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| YamlConfigError::IoError(e.to_string()))?;
        Self::from_str(&content)
    }

    /// Parse from a YAML string.
    pub fn from_str(yaml: &str) -> Result<Self, YamlConfigError> {
        serde_yaml::from_str(yaml).map_err(|e| YamlConfigError::ParseError(e.to_string()))
    }

    /// Convert mesh routes to MeshConfig.
    pub fn to_mesh_config(&self) -> MeshConfig {
        let routes = self
            .mesh
            .routes
            .iter()
            .map(|r| {
                let lane = match r.lane.to_lowercase().as_str() {
                    "trigger" => Lane::Trigger,
                    "control" => Lane::Control,
                    _ => Lane::Data,
                };
                MeshRoute {
                    from: r.from.clone(),
                    to: r.to.clone(),
                    lane,
                    mode: match lane {
                        Lane::Data => MeshMode::LoanWrite,
                        Lane::Control => MeshMode::Copy,
                        Lane::Trigger => MeshMode::LoanWrite,
                    },
                }
            })
            .collect();

        MeshConfig { routes }
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Check that all mesh route services exist in services list
        let service_names: std::collections::HashSet<&str> =
            self.services.iter().map(|s| s.name.as_str()).collect();

        for route in &self.mesh.routes {
            if !service_names.contains(route.from.as_str()) {
                errors.push(format!(
                    "Mesh route references unknown service '{}' (from)",
                    route.from
                ));
            }
            if !service_names.contains(route.to.as_str()) {
                errors.push(format!(
                    "Mesh route references unknown service '{}' (to)",
                    route.to
                ));
            }
        }

        // Check for duplicate service names
        let mut seen = std::collections::HashSet::new();
        for svc in &self.services {
            if !seen.insert(&svc.name) {
                errors.push(format!("Duplicate service name: '{}'", svc.name));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[derive(Debug)]
pub enum YamlConfigError {
    IoError(String),
    ParseError(String),
}

impl std::fmt::Display for YamlConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            YamlConfigError::IoError(e) => write!(f, "IO error: {}", e),
            YamlConfigError::ParseError(e) => write!(f, "YAML parse error: {}", e),
        }
    }
}

impl std::error::Error for YamlConfigError {}
