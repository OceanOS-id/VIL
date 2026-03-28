// =============================================================================
// VIL Server Config — Multi-source configuration with profiles
// =============================================================================
//
// Two config structs:
//   ServerConfig      — lightweight, for simple VX_APP servers
//   FullServerConfig  — comprehensive, for production vil-server deployments
//
// Configuration precedence: Code Default → YAML → Profile → ENV
// Profiles: dev (fast iteration), staging (validation), prod (P99 optimized)

pub mod gateway_config;
pub mod profiles;
pub mod server_config;
pub mod sources;

pub use gateway_config::GatewayConfig;
pub use profiles::Profile;
pub use server_config::FullServerConfig;

// Re-export infrastructure config types for downstream use
pub use server_config::{
    DatabaseSection, KafkaConfig, MqSection, MqttConfig, NatsConfig, PipelineSection,
    PostgresConfig, RedisConfig, ShmSection,
};

use serde::Deserialize;

/// Top-level server configuration (lightweight — for VX_APP examples).
/// For full production config, use FullServerConfig.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// Server settings
    pub server: ServerSection,
    /// Active profile name
    pub profile: String,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// SHM pool configuration
    pub shm: ShmConfig,
    /// Pipeline configuration
    pub pipeline: PipelineConfig,
    /// Service definitions (for unified mode)
    pub services: Vec<ServiceConfig>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server: ServerSection::default(),
            profile: "dev".to_string(),
            logging: LoggingConfig::default(),
            shm: ShmConfig::default(),
            pipeline: PipelineConfig::default(),
            services: Vec::new(),
        }
    }
}

/// Server network settings.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerSection {
    /// Main listening port
    pub port: u16,
    /// Bind address
    pub host: String,
    /// Worker thread count (0 = num_cpus)
    pub workers: usize,
    /// Separate metrics port (None = same as main)
    pub metrics_port: Option<u16>,
    /// Maximum request body size in bytes
    pub max_body_size: usize,
    /// Request timeout in seconds
    pub request_timeout_secs: u64,
}

impl Default for ServerSection {
    fn default() -> Self {
        Self {
            port: 8080,
            host: "0.0.0.0".to_string(),
            workers: 0,
            metrics_port: None,
            max_body_size: 10 * 1024 * 1024, // 10MB
            request_timeout_secs: 30,
        }
    }
}

/// Logging configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    /// Log level: trace, debug, info, warn, error
    pub level: String,
    /// Log format: text, json
    pub format: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "text".to_string(),
        }
    }
}

/// SHM pool configuration (lightweight version for ServerConfig).
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ShmConfig {
    /// Pool capacity in MB (default: 64)
    pub capacity_mb: usize,
    /// Reset when utilization exceeds this % (default: 85)
    pub reset_threshold_pct: usize,
    /// Check utilization every N allocations (default: 256)
    pub check_interval: u64,
}

impl Default for ShmConfig {
    fn default() -> Self {
        Self {
            capacity_mb: 64,
            reset_threshold_pct: 85,
            check_interval: 256,
        }
    }
}

impl ShmConfig {
    pub fn dev() -> Self {
        Self {
            capacity_mb: 8,
            reset_threshold_pct: 70,
            check_interval: 64,
        }
    }

    pub fn production() -> Self {
        Self {
            capacity_mb: 256,
            reset_threshold_pct: 90,
            check_interval: 1024,
        }
    }

    /// Capacity in bytes.
    pub fn capacity_bytes(&self) -> usize {
        self.capacity_mb * 1024 * 1024
    }
}

/// Pipeline configuration (lightweight version for ServerConfig).
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct PipelineConfig {
    /// Queue capacity for inter-process channels
    pub queue_capacity: usize,
    /// Session timeout in seconds
    pub session_timeout_secs: u64,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            queue_capacity: 1024,
            session_timeout_secs: 300,
        }
    }
}

/// Service definition for unified mode.
#[derive(Debug, Clone, Deserialize)]
pub struct ServiceConfig {
    /// Service name
    pub name: String,
    /// Route prefix
    pub prefix: Option<String>,
    /// Visibility: public or internal
    #[serde(default = "default_visibility")]
    pub visibility: String,
}

fn default_visibility() -> String {
    "public".to_string()
}

impl ServerConfig {
    /// Load configuration from a YAML file.
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: ServerConfig = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    /// Load configuration with profile presets + environment variable overrides.
    /// Precedence: YAML < Profile < ENV
    pub fn from_file_with_env(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut config = Self::from_file(path)?;
        config.apply_profile();
        config.apply_env_overrides();
        Ok(config)
    }

    /// Load from environment only (no file).
    pub fn from_env() -> Self {
        let mut config = Self::default();
        config.apply_env_overrides();
        config
    }

    /// Apply profile presets.
    pub fn apply_profile(&mut self) {
        let profile = Profile::from_str(&self.profile);
        match profile {
            Profile::Dev => {
                self.logging.level = "debug".into();
                self.shm = ShmConfig::dev();
                self.pipeline.queue_capacity = 256;
                self.pipeline.session_timeout_secs = 60;
            }
            Profile::Staging => {
                self.logging.level = "info".into();
                self.logging.format = "json".into();
            }
            Profile::Prod => {
                self.logging.level = "warn".into();
                self.logging.format = "json".into();
                self.shm = ShmConfig::production();
                self.pipeline.queue_capacity = 4096;
                self.pipeline.session_timeout_secs = 600;
            }
            Profile::Custom(_) => {}
        }
    }

    /// Apply VIL_* environment variable overrides.
    fn apply_env_overrides(&mut self) {
        if let Ok(port) = std::env::var("VIL_SERVER_PORT") {
            if let Ok(p) = port.parse() {
                self.server.port = p;
            }
        }
        if let Ok(host) = std::env::var("VIL_SERVER_HOST") {
            self.server.host = host;
        }
        if let Ok(level) = std::env::var("VIL_LOG_LEVEL") {
            self.logging.level = level;
        }
        if let Ok(profile) = std::env::var("VIL_PROFILE") {
            self.profile = profile;
        }
        if let Ok(workers) = std::env::var("VIL_SERVER_WORKERS") {
            if let Ok(w) = workers.parse() {
                self.server.workers = w;
            }
        }
        if let Ok(mp) = std::env::var("VIL_METRICS_PORT") {
            if let Ok(p) = mp.parse() {
                self.server.metrics_port = Some(p);
            }
        }
        // SHM
        if let Ok(mb) = std::env::var("VIL_SHM_CAPACITY_MB") {
            if let Ok(v) = mb.parse() {
                self.shm.capacity_mb = v;
            }
        }
        if let Ok(pct) = std::env::var("VIL_SHM_RESET_PCT") {
            if let Ok(v) = pct.parse::<usize>() {
                self.shm.reset_threshold_pct = v.min(99);
            }
        }
        if let Ok(interval) = std::env::var("VIL_SHM_CHECK_INTERVAL") {
            if let Ok(v) = interval.parse::<u64>() {
                self.shm.check_interval = v.max(1);
            }
        }
        // Pipeline
        if let Ok(v) = std::env::var("VIL_PIPELINE_QUEUE_CAPACITY") {
            if let Ok(c) = v.parse() {
                self.pipeline.queue_capacity = c;
            }
        }
        if let Ok(v) = std::env::var("VIL_PIPELINE_SESSION_TIMEOUT") {
            if let Ok(t) = v.parse() {
                self.pipeline.session_timeout_secs = t;
            }
        }
    }
}
