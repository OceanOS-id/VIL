// =============================================================================
// vil_log builder — ergonomic init with dev/prod toggle
// =============================================================================

use crate::config::LogConfig;
use crate::drain::{StdoutDrain, StdoutFormat};
use crate::runtime;
use crate::types::LogLevel;

/// Builder for vil_log initialization.
///
/// # Example
/// ```ignore
/// let _log = vil_log::init()
///     .dev_mode(cfg!(debug_assertions))
///     .stdout(vil_log::StdoutFormat::Pretty)
///     .build();
/// ```
pub struct VilLogBuilder {
    config: LogConfig,
    drain_format: StdoutFormat,
    dev_mode: bool,
}

impl VilLogBuilder {
    pub fn new() -> Self {
        Self {
            config: LogConfig {
                ring_slots: 4096,
                level: LogLevel::Info,
                batch_size: 100,
                flush_interval_ms: 200,
                threads: None,
                dict_path: None,
                fallback_path: None,
                drain_failure_threshold: 3,
            },
            drain_format: StdoutFormat::Pretty,
            dev_mode: false,
        }
    }

    /// Set log level.
    pub fn level(mut self, level: LogLevel) -> Self {
        self.config.level = level;
        self
    }

    /// Set SPSC ring buffer slots (production mode only).
    pub fn ring_slots(mut self, slots: usize) -> Self {
        self.config.ring_slots = slots;
        self
    }

    /// Set flush interval in milliseconds (production mode only).
    pub fn flush_interval_ms(mut self, ms: u64) -> Self {
        self.config.flush_interval_ms = ms;
        self
    }

    /// Toggle development mode.
    ///
    /// - `true`: skip SPSC init, fallback to tracing (colored terminal)
    /// - `false`: full SPSC ring buffer + drain task (fast, structured)
    ///
    /// Tip: `cfg!(debug_assertions)` auto-detects build profile.
    pub fn dev_mode(mut self, enabled: bool) -> Self {
        self.dev_mode = enabled;
        self
    }

    /// Set stdout drain format (production mode).
    pub fn stdout(mut self, format: StdoutFormat) -> Self {
        self.drain_format = format;
        self
    }

    /// Build and initialize the logging system.
    pub fn build(self) -> VilLogGuard {
        if self.dev_mode {
            // Dev mode: tracing-subscriber for colored terminal
            let level = match self.config.level {
                LogLevel::Trace => "trace",
                LogLevel::Debug => "debug",
                LogLevel::Info => "info",
                LogLevel::Warn => "warn",
                LogLevel::Error => "error",
                _ => "info",
            };
            let _ = tracing_subscriber::fmt()
                .with_env_filter(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| level.into()),
                )
                .try_init();
            // All vil_log macros auto-fallback to tracing when ring not initialized
            return VilLogGuard { _task: None };
        }

        // Production: SPSC ring buffer + stdout drain
        let drain = StdoutDrain::new(self.drain_format);
        let task = runtime::init_logging(self.config, drain);
        VilLogGuard { _task: Some(task) }
    }
}

impl Default for VilLogBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Guard that keeps vil_log alive. Drop = flush.
pub struct VilLogGuard {
    _task: Option<tokio::task::JoinHandle<()>>,
}
