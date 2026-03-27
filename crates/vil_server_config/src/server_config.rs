// =============================================================================
// vil-server.yaml — Full Server Framework Configuration
// =============================================================================
//
// Every configurable value in vil-server. No hardcoded defaults in code.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Top-level vil-server.yaml.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FullServerConfig {
    /// Active profile: dev, staging, prod
    #[serde(default = "default_profile")]
    pub profile: String,
    pub server: ServerSection,
    pub logging: LogSection,
    pub shm: ShmSection,
    pub mesh: MeshSection,
    pub pipeline: PipelineSection,
    pub database: DatabaseSection,
    pub mq: MqSection,
    pub services: Vec<ServiceSection>,
    pub middleware: MiddlewareSection,
    pub security: SecuritySection,
    pub session: SessionSection,
    pub observability: ObservabilitySection,
    pub performance: PerformanceSection,
    pub grpc: GrpcServerSection,
    pub graphql: GraphqlSection,
    pub feature_flags: FeatureFlagsSection,
    pub scheduler: SchedulerSection,
    pub plugins: PluginsSection,
    pub rolling_restart: RollingRestartSection,
    pub admin: AdminSection,
}

fn default_profile() -> String { "dev".into() }

// ==================== Server ====================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerSection {
    pub name: String,
    pub port: u16,
    pub host: String,
    pub metrics_port: Option<u16>,
    pub workers: usize,
    pub request_timeout_secs: u64,
    pub max_body_size: String,
    pub graceful_shutdown_timeout_secs: u64,
}

impl Default for ServerSection {
    fn default() -> Self {
        Self {
            name: "vil-server".into(), port: 8080, host: "0.0.0.0".into(),
            metrics_port: None, workers: 0, request_timeout_secs: 30,
            max_body_size: "1MB".into(), graceful_shutdown_timeout_secs: 30,
        }
    }
}

// ==================== Logging ====================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LogSection {
    pub level: String,
    pub format: String,
    #[serde(default)]
    pub modules: HashMap<String, String>,
}

impl Default for LogSection {
    fn default() -> Self {
        Self { level: "info".into(), format: "text".into(), modules: HashMap::new() }
    }
}

// ==================== SHM ====================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ShmSection {
    pub enabled: bool,
    pub pool_size: String,
    pub reset_threshold_pct: usize,
    /// Amortized reset check interval (every N allocs). Higher = better P99.
    pub check_interval: u64,
    pub query_cache: QueryCacheSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct QueryCacheSection {
    pub enabled: bool,
    pub region_size: String,
    pub default_ttl_secs: u64,
    pub max_entries: usize,
}

impl Default for ShmSection {
    fn default() -> Self {
        Self {
            enabled: true, pool_size: "64MB".into(), reset_threshold_pct: 85,
            check_interval: 256,
            query_cache: QueryCacheSection::default(),
        }
    }
}

impl Default for QueryCacheSection {
    fn default() -> Self {
        Self { enabled: true, region_size: "32MB".into(), default_ttl_secs: 60, max_entries: 10000 }
    }
}

// ==================== Mesh ====================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MeshSection {
    pub mode: String,
    pub channels: ChannelsSection,
    pub discovery: DiscoverySection,
    #[serde(default)]
    pub routes: Vec<RouteSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ChannelsSection {
    pub trigger: ChannelConfig,
    pub data: ChannelConfig,
    pub control: ChannelConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ChannelConfig {
    pub buffer_size: usize,
    pub shm_region_size: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DiscoverySection {
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteSection {
    pub from: String,
    pub to: String,
    #[serde(default = "default_lane")]
    pub lane: String,
}

fn default_lane() -> String { "data".into() }

impl Default for MeshSection {
    fn default() -> Self {
        Self {
            mode: "unified".into(), channels: ChannelsSection::default(),
            discovery: DiscoverySection::default(), routes: Vec::new(),
        }
    }
}

impl Default for ChannelsSection {
    fn default() -> Self {
        Self {
            trigger: ChannelConfig { buffer_size: 1024, shm_region_size: "4MB".into() },
            data: ChannelConfig { buffer_size: 1024, shm_region_size: "16MB".into() },
            control: ChannelConfig { buffer_size: 256, shm_region_size: "1MB".into() },
        }
    }
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self { buffer_size: 1024, shm_region_size: "4MB".into() }
    }
}

impl Default for DiscoverySection {
    fn default() -> Self {
        Self { mode: "shm".into() }
    }
}

// ==================== Services ====================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceSection {
    pub name: String,
    #[serde(default = "default_public")]
    pub visibility: String,
    #[serde(default)]
    pub prefix: Option<String>,
}

fn default_public() -> String { "public".into() }

// ==================== Middleware ====================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MiddlewareSection {
    pub request_tracker: ToggleConfig,
    pub handler_metrics: SampledConfig,
    pub tracing: TracingConfig,
    pub cors: CorsConfig,
    pub compression: CompressionConfig,
    pub timeout: TimeoutConfig,
    pub security_headers: ToggleConfig,
    pub hsts: HstsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ToggleConfig { pub enabled: bool }
impl Default for ToggleConfig { fn default() -> Self { Self { enabled: true } } }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SampledConfig { pub enabled: bool, pub sample_rate: u64 }
impl Default for SampledConfig { fn default() -> Self { Self { enabled: true, sample_rate: 1 } } }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TracingConfig { pub enabled: bool, pub sample_rate: u64, pub propagation: String }
impl Default for TracingConfig { fn default() -> Self { Self { enabled: true, sample_rate: 1, propagation: "w3c".into() } } }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CorsConfig { pub enabled: bool, pub mode: String }
impl Default for CorsConfig { fn default() -> Self { Self { enabled: true, mode: "permissive".into() } } }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CompressionConfig { pub enabled: bool, pub min_body_size: usize }
impl Default for CompressionConfig { fn default() -> Self { Self { enabled: false, min_body_size: 256 } } }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TimeoutConfig { pub enabled: bool, pub duration_secs: u64 }
impl Default for TimeoutConfig { fn default() -> Self { Self { enabled: true, duration_secs: 30 } } }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HstsConfig { pub enabled: bool, pub max_age_secs: u64, pub include_subdomains: bool }
impl Default for HstsConfig { fn default() -> Self { Self { enabled: false, max_age_secs: 31536000, include_subdomains: true } } }

impl Default for MiddlewareSection {
    fn default() -> Self {
        Self {
            request_tracker: ToggleConfig { enabled: true },
            handler_metrics: SampledConfig::default(),
            tracing: TracingConfig::default(),
            cors: CorsConfig::default(),
            compression: CompressionConfig::default(),
            timeout: TimeoutConfig::default(),
            security_headers: ToggleConfig { enabled: true },
            hsts: HstsConfig::default(),
        }
    }
}

// ==================== Security ====================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SecuritySection {
    pub jwt: JwtConfig,
    pub rate_limit: RateLimitConfig,
    pub csrf: CsrfConfig,
    pub brute_force: BruteForceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct JwtConfig { pub enabled: bool, pub secret: String, pub algorithm: String, pub optional: bool }
impl Default for JwtConfig { fn default() -> Self { Self { enabled: false, secret: String::new(), algorithm: "HS256".into(), optional: false } } }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RateLimitConfig { pub enabled: bool, pub max_requests: u64, pub window_secs: u64, pub per: String }
impl Default for RateLimitConfig { fn default() -> Self { Self { enabled: false, max_requests: 1000, window_secs: 60, per: "ip".into() } } }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CsrfConfig { pub enabled: bool, pub cookie_name: String, #[serde(default)] pub exempt_paths: Vec<String> }
impl Default for CsrfConfig { fn default() -> Self { Self { enabled: false, cookie_name: "vil-csrf-token".into(), exempt_paths: Vec::new() } } }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BruteForceConfig { pub enabled: bool, pub max_attempts: u64, pub block_duration_secs: u64 }
impl Default for BruteForceConfig { fn default() -> Self { Self { enabled: false, max_attempts: 5, block_duration_secs: 300 } } }

impl Default for SecuritySection {
    fn default() -> Self {
        Self { jwt: JwtConfig::default(), rate_limit: RateLimitConfig::default(), csrf: CsrfConfig::default(), brute_force: BruteForceConfig::default() }
    }
}

// ==================== Session ====================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionSection {
    pub enabled: bool, pub cookie_name: String, pub ttl_secs: u64,
    pub http_only: bool, pub secure: bool, pub same_site: String,
}

impl Default for SessionSection {
    fn default() -> Self {
        Self { enabled: false, cookie_name: "vil-session".into(), ttl_secs: 1800, http_only: true, secure: false, same_site: "Lax".into() }
    }
}

// ==================== Observability ====================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ObservabilitySection {
    pub error_tracker: ErrorTrackerConfig,
    pub span_collector: SpanCollectorConfig,
    pub profiler: ToggleConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ErrorTrackerConfig { pub enabled: bool, pub max_recent: usize }
impl Default for ErrorTrackerConfig { fn default() -> Self { Self { enabled: true, max_recent: 1000 } } }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SpanCollectorConfig { pub max_spans: usize }
impl Default for SpanCollectorConfig { fn default() -> Self { Self { max_spans: 10000 } } }

impl Default for ObservabilitySection {
    fn default() -> Self {
        Self { error_tracker: ErrorTrackerConfig::default(), span_collector: SpanCollectorConfig::default(), profiler: ToggleConfig { enabled: true } }
    }
}

// ==================== Performance ====================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PerformanceSection {
    pub metrics_sample_rate: u64,
    pub trace_sample_rate: u64,
    pub idempotency: IdempotencyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct IdempotencyConfig { pub enabled: bool, pub ttl_secs: u64, pub max_entries: usize }
impl Default for IdempotencyConfig { fn default() -> Self { Self { enabled: false, ttl_secs: 86400, max_entries: 10000 } } }

impl Default for PerformanceSection {
    fn default() -> Self {
        Self { metrics_sample_rate: 1, trace_sample_rate: 1, idempotency: IdempotencyConfig::default() }
    }
}

// ==================== gRPC ====================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GrpcServerSection {
    pub enabled: bool, pub port: u16, pub max_message_size: String,
    pub health_check: bool, pub reflection: bool, pub max_concurrent_streams: u32,
}

impl Default for GrpcServerSection {
    fn default() -> Self {
        Self { enabled: false, port: 50051, max_message_size: "4MB".into(), health_check: true, reflection: true, max_concurrent_streams: 200 }
    }
}

// ==================== GraphQL ====================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphqlSection {
    pub enabled: bool, pub playground: bool, pub max_depth: usize,
    pub max_complexity: usize, pub introspection: bool,
    pub default_page_size: usize, pub max_page_size: usize,
}

impl Default for GraphqlSection {
    fn default() -> Self {
        Self { enabled: false, playground: true, max_depth: 10, max_complexity: 1000, introspection: true, default_page_size: 20, max_page_size: 100 }
    }
}

// ==================== Feature Flags ====================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FeatureFlagsSection {
    #[serde(default)]
    pub flags: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub file: Option<String>,
}

impl Default for FeatureFlagsSection {
    fn default() -> Self { Self { flags: HashMap::new(), file: None } }
}

// ==================== Scheduler ====================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SchedulerSection {
    #[serde(default)]
    pub jobs: Vec<JobConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobConfig {
    pub name: String,
    pub every_secs: u64,
}

impl Default for SchedulerSection {
    fn default() -> Self { Self { jobs: Vec::new() } }
}

// ==================== Plugins ====================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginsSection {
    pub directory: String,
    pub encryption_key: String,
    #[serde(default)]
    pub active: Vec<PluginRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRef {
    pub name: String,
    #[serde(default)]
    pub config_file: Option<String>,
}

impl Default for PluginsSection {
    fn default() -> Self {
        Self {
            directory: "~/.vil/plugins".into(),
            encryption_key: "~/.vil/secrets/encryption.key".into(),
            active: Vec::new(),
        }
    }
}

// ==================== Rolling Restart ====================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RollingRestartSection {
    pub drain_timeout_secs: u64,
}

impl Default for RollingRestartSection {
    fn default() -> Self { Self { drain_timeout_secs: 30 } }
}

// ==================== Admin ====================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AdminSection {
    pub playground: bool,
    pub diagnostics: bool,
    pub hot_reload: bool,
    pub plugin_gui: bool,
}

impl Default for AdminSection {
    fn default() -> Self {
        Self { playground: true, diagnostics: true, hot_reload: true, plugin_gui: true }
    }
}

// ==================== Pipeline ====================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PipelineSection {
    /// Queue capacity for inter-process channels
    pub queue_capacity: usize,
    /// Session timeout in seconds (idle sessions reclaimed)
    pub session_timeout_secs: u64,
    /// Maximum concurrent pipelines
    pub max_concurrent: usize,
}

impl Default for PipelineSection {
    fn default() -> Self {
        Self { queue_capacity: 1024, session_timeout_secs: 300, max_concurrent: 64 }
    }
}

// ==================== Database ====================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DatabaseSection {
    pub postgres: PostgresConfig,
    pub redis: RedisConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PostgresConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout_secs: u64,
    pub idle_timeout_secs: u64,
    pub max_lifetime_secs: u64,
}

impl Default for PostgresConfig {
    fn default() -> Self {
        Self {
            url: "postgres://vil:vil@localhost:5432/vil".into(),
            max_connections: 10,
            min_connections: 1,
            connect_timeout_secs: 5,
            idle_timeout_secs: 300,
            max_lifetime_secs: 1800,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RedisConfig {
    pub url: String,
    pub pool_size: u32,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self { url: "redis://localhost:6380".into(), pool_size: 4 }
    }
}

impl Default for DatabaseSection {
    fn default() -> Self {
        Self { postgres: PostgresConfig::default(), redis: RedisConfig::default() }
    }
}

// ==================== Message Queues ====================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MqSection {
    pub nats: NatsConfig,
    pub kafka: KafkaConfig,
    pub mqtt: MqttConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NatsConfig {
    pub url: String,
    pub max_reconnects: Option<usize>,
}

impl Default for NatsConfig {
    fn default() -> Self {
        Self { url: "nats://localhost:4222".into(), max_reconnects: Some(60) }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KafkaConfig {
    pub brokers: String,
    pub group_id: String,
}

impl Default for KafkaConfig {
    fn default() -> Self {
        Self { brokers: "localhost:9092".into(), group_id: "vil-default".into() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MqttConfig {
    pub host: String,
    pub port: u16,
    pub client_id: String,
    pub keep_alive_secs: u64,
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self { host: "localhost".into(), port: 1883, client_id: "vil-client".into(), keep_alive_secs: 30 }
    }
}

impl Default for MqSection {
    fn default() -> Self {
        Self { nats: NatsConfig::default(), kafka: KafkaConfig::default(), mqtt: MqttConfig::default() }
    }
}

// ==================== Top-level default ====================
impl Default for FullServerConfig {
    fn default() -> Self {
        Self {
            profile: "dev".into(),
            server: ServerSection::default(),
            logging: LogSection::default(),
            shm: ShmSection::default(),
            mesh: MeshSection::default(),
            pipeline: PipelineSection::default(),
            database: DatabaseSection::default(),
            mq: MqSection::default(),
            services: Vec::new(),
            middleware: MiddlewareSection::default(),
            security: SecuritySection::default(),
            session: SessionSection::default(),
            observability: ObservabilitySection::default(),
            performance: PerformanceSection::default(),
            grpc: GrpcServerSection::default(),
            graphql: GraphqlSection::default(),
            feature_flags: FeatureFlagsSection::default(),
            scheduler: SchedulerSection::default(),
            plugins: PluginsSection::default(),
            rolling_restart: RollingRestartSection::default(),
            admin: AdminSection::default(),
        }
    }
}

// ==================== Load ====================
impl FullServerConfig {
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        Self::from_str(&content)
    }

    pub fn from_str(yaml: &str) -> Result<Self, String> {
        serde_yaml::from_str(yaml).map_err(|e| format!("Parse error: {}", e))
    }

    /// Load with profile presets + environment variable overrides.
    /// Precedence: YAML < Profile < ENV
    pub fn from_file_with_env(path: &Path) -> Result<Self, String> {
        let mut config = Self::from_file(path)?;
        config.apply_profile();
        config.apply_env_overrides();
        Ok(config)
    }

    /// Apply VIL_* environment variable overrides.
    pub fn apply_env_overrides(&mut self) {
        if let Ok(v) = std::env::var("VIL_SERVER_PORT") {
            if let Ok(p) = v.parse() { self.server.port = p; }
        }
        if let Ok(v) = std::env::var("VIL_SERVER_HOST") { self.server.host = v; }
        if let Ok(v) = std::env::var("VIL_METRICS_PORT") {
            if let Ok(p) = v.parse() { self.server.metrics_port = Some(p); }
        }
        if let Ok(v) = std::env::var("VIL_LOG_LEVEL") { self.logging.level = v; }
        if let Ok(v) = std::env::var("VIL_LOG_FORMAT") { self.logging.format = v; }
        if let Ok(v) = std::env::var("VIL_WORKERS") {
            if let Ok(w) = v.parse() { self.server.workers = w; }
        }
        if let Ok(v) = std::env::var("VIL_REQUEST_TIMEOUT") {
            if let Ok(t) = v.parse() { self.server.request_timeout_secs = t; }
        }
        if let Ok(v) = std::env::var("VIL_SHM_POOL_SIZE") { self.shm.pool_size = v; }
        if let Ok(v) = std::env::var("VIL_SHM_RESET_PCT") {
            if let Ok(p) = v.parse() { self.shm.reset_threshold_pct = p; }
        }
        if let Ok(v) = std::env::var("VIL_SHM_CHECK_INTERVAL") {
            if let Ok(i) = v.parse() { self.shm.check_interval = i; }
        }
        // Database
        if let Ok(v) = std::env::var("VIL_DATABASE_URL") { self.database.postgres.url = v; }
        if let Ok(v) = std::env::var("VIL_DATABASE_MAX_CONNECTIONS") {
            if let Ok(c) = v.parse() { self.database.postgres.max_connections = c; }
        }
        if let Ok(v) = std::env::var("VIL_REDIS_URL") { self.database.redis.url = v; }
        // Message Queues
        if let Ok(v) = std::env::var("VIL_NATS_URL") { self.mq.nats.url = v; }
        if let Ok(v) = std::env::var("VIL_KAFKA_BROKERS") { self.mq.kafka.brokers = v; }
        if let Ok(v) = std::env::var("VIL_MQTT_HOST") { self.mq.mqtt.host = v; }
        if let Ok(v) = std::env::var("VIL_MQTT_PORT") {
            if let Ok(p) = v.parse() { self.mq.mqtt.port = p; }
        }
        // Pipeline
        if let Ok(v) = std::env::var("VIL_PIPELINE_QUEUE_CAPACITY") {
            if let Ok(c) = v.parse() { self.pipeline.queue_capacity = c; }
        }
        if let Ok(v) = std::env::var("VIL_PIPELINE_SESSION_TIMEOUT") {
            if let Ok(t) = v.parse() { self.pipeline.session_timeout_secs = t; }
        }
        // Profile
        if let Ok(v) = std::env::var("VIL_PROFILE") {
            self.profile = v;
        }
    }

    /// Apply profile presets. Call after loading YAML but before env overrides
    /// if you want env to take final precedence.
    pub fn apply_profile(&mut self) {
        use crate::profiles::Profile;
        let profile = Profile::from_str(&self.profile);
        profile.apply(self);
    }

    /// Parse size string like "64MB" to bytes.
    pub fn parse_size(s: &str) -> usize {
        let s = s.trim();
        if let Some(mb) = s.strip_suffix("MB") {
            mb.trim().parse::<usize>().unwrap_or(64) * 1024 * 1024
        } else if let Some(kb) = s.strip_suffix("KB") {
            kb.trim().parse::<usize>().unwrap_or(1024) * 1024
        } else if let Some(gb) = s.strip_suffix("GB") {
            gb.trim().parse::<usize>().unwrap_or(1) * 1024 * 1024 * 1024
        } else {
            s.parse::<usize>().unwrap_or(67108864) // default 64MB
        }
    }
}
