// =============================================================================
// VIL Server Config Sources — File, ENV, CLI
// =============================================================================

/// Configuration source types in order of precedence (lowest to highest).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigSource {
    /// Default values
    Default,
    /// YAML/TOML configuration file
    File,
    /// Environment variables
    Environment,
    /// CLI flags
    CommandLine,
}

impl std::fmt::Display for ConfigSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigSource::Default => write!(f, "default"),
            ConfigSource::File => write!(f, "file"),
            ConfigSource::Environment => write!(f, "environment"),
            ConfigSource::CommandLine => write!(f, "cli"),
        }
    }
}
