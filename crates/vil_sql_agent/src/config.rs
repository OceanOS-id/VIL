use serde::{Deserialize, Serialize};

/// Configuration for SQL generation and validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlAgentConfig {
    /// Whether to allow write operations (INSERT, UPDATE, DELETE).
    pub allow_writes: bool,
    /// Whether to allow DDL operations (CREATE, ALTER, DROP).
    pub allow_ddl: bool,
    /// Maximum result limit to enforce.
    pub max_limit: usize,
    /// Default parameter placeholder style ("$1" for Postgres, "?" for MySQL/SQLite).
    pub placeholder_style: PlaceholderStyle,
}

/// Placeholder style for parameterized queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlaceholderStyle {
    /// PostgreSQL style: $1, $2, $3
    Dollar,
    /// MySQL/SQLite style: ?
    QuestionMark,
    /// Named style: :name
    Named,
}

impl Default for SqlAgentConfig {
    fn default() -> Self {
        Self {
            allow_writes: false,
            allow_ddl: false,
            max_limit: 1000,
            placeholder_style: PlaceholderStyle::Dollar,
        }
    }
}

impl SqlAgentConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn allow_writes(mut self, allow: bool) -> Self {
        self.allow_writes = allow;
        self
    }

    pub fn allow_ddl(mut self, allow: bool) -> Self {
        self.allow_ddl = allow;
        self
    }

    pub fn max_limit(mut self, limit: usize) -> Self {
        self.max_limit = limit;
        self
    }

    pub fn placeholder_style(mut self, style: PlaceholderStyle) -> Self {
        self.placeholder_style = style;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let cfg = SqlAgentConfig::default();
        assert!(!cfg.allow_writes);
        assert!(!cfg.allow_ddl);
        assert_eq!(cfg.max_limit, 1000);
    }

    #[test]
    fn test_config_builder() {
        let cfg = SqlAgentConfig::new()
            .allow_writes(true)
            .allow_ddl(false)
            .max_limit(500)
            .placeholder_style(PlaceholderStyle::QuestionMark);

        assert!(cfg.allow_writes);
        assert!(!cfg.allow_ddl);
        assert_eq!(cfg.max_limit, 500);
    }
}
