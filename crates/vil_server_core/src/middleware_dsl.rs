// =============================================================================
// VIL Server — Declarative Middleware Pipeline DSL
// =============================================================================
//
// Define middleware stacks via YAML configuration.
// This allows middleware to be configured without recompilation.
//
// Example vil-server.yaml:
//   middleware:
//     - name: timeout
//       config:
//         duration_secs: 30
//     - name: compression
//     - name: security_headers
//     - name: rate_limit
//       config:
//         max_requests: 1000
//         window_secs: 60
//     - name: jwt_auth
//       config:
//         secret: ${JWT_SECRET}
//         optional: true

use serde::Deserialize;

/// Middleware definition from YAML.
#[derive(Debug, Clone, Deserialize)]
pub struct MiddlewareDef {
    /// Middleware name (must match a registered middleware)
    pub name: String,
    /// Optional configuration (middleware-specific)
    #[serde(default)]
    pub config: serde_json::Value,
    /// Whether this middleware is enabled (default: true)
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Apply only to specific paths (empty = all paths)
    #[serde(default)]
    pub paths: Vec<String>,
}

fn default_true() -> bool {
    true
}

/// Middleware pipeline definition from YAML.
#[derive(Debug, Clone, Deserialize)]
pub struct MiddlewarePipeline {
    /// Ordered list of middleware to apply
    #[serde(default)]
    pub middleware: Vec<MiddlewareDef>,
}

impl MiddlewarePipeline {
    /// Parse from YAML string.
    pub fn from_yaml(yaml: &str) -> Result<Self, String> {
        serde_yaml::from_str(yaml).map_err(|e| format!("Failed to parse middleware YAML: {}", e))
    }

    /// Get enabled middleware in order.
    pub fn enabled(&self) -> Vec<&MiddlewareDef> {
        self.middleware.iter().filter(|m| m.enabled).collect()
    }

    /// Validate that all middleware names are recognized.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let known = [
            "timeout",
            "compression",
            "security_headers",
            "cors",
            "rate_limit",
            "jwt_auth",
            "api_key",
            "csrf",
            "request_logging",
            "handler_metrics",
            "tracing",
            "ip_filter",
            "brute_force",
            "hsts",
        ];

        let mut errors = Vec::new();
        for mw in &self.middleware {
            if !known.contains(&mw.name.as_str()) {
                errors.push(format!(
                    "Unknown middleware: '{}'. Known: {:?}",
                    mw.name, known
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Get middleware count.
    pub fn count(&self) -> usize {
        self.middleware.len()
    }
}

impl Default for MiddlewarePipeline {
    fn default() -> Self {
        Self {
            middleware: vec![
                MiddlewareDef {
                    name: "handler_metrics".to_string(),
                    config: serde_json::Value::Null,
                    enabled: true,
                    paths: Vec::new(),
                },
                MiddlewareDef {
                    name: "request_logging".to_string(),
                    config: serde_json::Value::Null,
                    enabled: true,
                    paths: Vec::new(),
                },
                MiddlewareDef {
                    name: "cors".to_string(),
                    config: serde_json::Value::Null,
                    enabled: true,
                    paths: Vec::new(),
                },
            ],
        }
    }
}
