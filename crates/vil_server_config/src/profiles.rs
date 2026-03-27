// =============================================================================
// VIL Server Config Profiles — dev, staging, prod
// =============================================================================
//
// Each profile applies tuned defaults for its target environment.
// Precedence: Code Default → YAML → Profile → ENV
//
// Profile presets are applied AFTER YAML loading, so YAML values act as
// the base that profiles selectively override. ENV always wins.

use crate::server_config::FullServerConfig;

/// Built-in configuration profiles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Profile {
    Dev,
    Staging,
    Prod,
    Custom(String),
}

impl Profile {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "dev" | "development" => Profile::Dev,
            "staging" | "stage" => Profile::Staging,
            "prod" | "production" => Profile::Prod,
            other => Profile::Custom(other.to_string()),
        }
    }

    pub fn is_dev(&self) -> bool {
        matches!(self, Profile::Dev)
    }

    pub fn is_prod(&self) -> bool {
        matches!(self, Profile::Prod)
    }

    pub fn default_log_level(&self) -> &str {
        match self {
            Profile::Dev => "debug",
            Profile::Staging => "info",
            Profile::Prod => "warn",
            Profile::Custom(_) => "info",
        }
    }

    /// Apply profile-specific tuning to a FullServerConfig.
    /// Only overrides values that differ from the generic defaults.
    pub fn apply(&self, config: &mut FullServerConfig) {
        match self {
            Profile::Dev => self.apply_dev(config),
            Profile::Staging => self.apply_staging(config),
            Profile::Prod => self.apply_prod(config),
            Profile::Custom(_) => {} // No-op for custom profiles
        }
    }

    fn apply_dev(&self, config: &mut FullServerConfig) {
        // ── Logging ──
        config.logging.level = "debug".into();
        config.logging.format = "text".into();

        // ── SHM — small pool, aggressive reset for fast iteration ──
        config.shm.pool_size = "8MB".into();
        config.shm.reset_threshold_pct = 70;
        config.shm.check_interval = 64;
        config.shm.query_cache.region_size = "4MB".into();
        config.shm.query_cache.max_entries = 1000;

        // ── Mesh — smaller buffers ──
        config.mesh.channels.trigger.buffer_size = 256;
        config.mesh.channels.data.buffer_size = 256;
        config.mesh.channels.control.buffer_size = 64;

        // ── Pipeline — fast timeout for dev ──
        config.pipeline.queue_capacity = 256;
        config.pipeline.session_timeout_secs = 60;
        config.pipeline.max_concurrent = 16;

        // ── Database — minimal pool ──
        config.database.postgres.max_connections = 5;
        config.database.postgres.min_connections = 1;

        // ── Admin — all enabled for development ──
        config.admin.playground = true;
        config.admin.diagnostics = true;
        config.admin.hot_reload = true;
        config.admin.plugin_gui = true;

        // ── Performance — full tracing ──
        config.performance.metrics_sample_rate = 1;
        config.performance.trace_sample_rate = 1;
    }

    fn apply_staging(&self, config: &mut FullServerConfig) {
        // ── Logging ──
        config.logging.level = "info".into();
        config.logging.format = "json".into();

        // ── SHM — moderate pool ──
        config.shm.pool_size = "64MB".into();
        config.shm.reset_threshold_pct = 85;
        config.shm.check_interval = 256;
        config.shm.query_cache.region_size = "16MB".into();
        config.shm.query_cache.max_entries = 5000;

        // ── Mesh — standard buffers ──
        config.mesh.channels.trigger.buffer_size = 1024;
        config.mesh.channels.data.buffer_size = 1024;
        config.mesh.channels.control.buffer_size = 256;

        // ── Pipeline ──
        config.pipeline.queue_capacity = 1024;
        config.pipeline.session_timeout_secs = 300;
        config.pipeline.max_concurrent = 64;

        // ── Database — moderate pool ──
        config.database.postgres.max_connections = 20;
        config.database.postgres.min_connections = 5;

        // ── Admin — selective ──
        config.admin.playground = true;
        config.admin.diagnostics = true;
        config.admin.hot_reload = false;
        config.admin.plugin_gui = true;

        // ── Security — rate limit on ──
        config.security.rate_limit.enabled = true;
        config.security.rate_limit.max_requests = 5000;

        // ── Performance — sampled tracing ──
        config.performance.metrics_sample_rate = 1;
        config.performance.trace_sample_rate = 10;
    }

    fn apply_prod(&self, config: &mut FullServerConfig) {
        // ── Logging ──
        config.logging.level = "warn".into();
        config.logging.format = "json".into();

        // ── SHM — large pool, high threshold, infrequent check for P99 ──
        config.shm.pool_size = "256MB".into();
        config.shm.reset_threshold_pct = 90;
        config.shm.check_interval = 1024;
        config.shm.query_cache.region_size = "64MB".into();
        config.shm.query_cache.max_entries = 50000;

        // ── Mesh — large buffers for throughput ──
        config.mesh.channels.trigger.buffer_size = 4096;
        config.mesh.channels.data.buffer_size = 4096;
        config.mesh.channels.control.buffer_size = 1024;

        // ── Pipeline — high capacity ──
        config.pipeline.queue_capacity = 4096;
        config.pipeline.session_timeout_secs = 600;
        config.pipeline.max_concurrent = 256;

        // ── Database — full pool ──
        config.database.postgres.max_connections = 50;
        config.database.postgres.min_connections = 10;
        config.database.postgres.idle_timeout_secs = 600;
        config.database.postgres.max_lifetime_secs = 3600;
        config.database.redis.pool_size = 16;

        // ── Admin — locked down ──
        config.admin.playground = false;
        config.admin.diagnostics = false;
        config.admin.hot_reload = false;
        config.admin.plugin_gui = false;

        // ── Security — hardened ──
        config.security.rate_limit.enabled = true;
        config.security.rate_limit.max_requests = 1000;
        config.security.rate_limit.window_secs = 60;
        config.session.secure = true;
        config.session.same_site = "Strict".into();
        config.middleware.hsts.enabled = true;
        config.middleware.compression.enabled = true;

        // ── Performance — aggressive sampling ──
        config.performance.metrics_sample_rate = 10;
        config.performance.trace_sample_rate = 100;
    }
}

impl std::fmt::Display for Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Profile::Dev => write!(f, "dev"),
            Profile::Staging => write!(f, "staging"),
            Profile::Prod => write!(f, "prod"),
            Profile::Custom(s) => write!(f, "{}", s),
        }
    }
}
