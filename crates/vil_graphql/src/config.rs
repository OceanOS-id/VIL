use serde::{Deserialize, Serialize};

/// GraphQL plugin configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLConfig {
    /// Enable /graphql endpoint
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Enable /graphql/playground (GraphiQL)
    #[serde(default = "default_true")]
    pub playground: bool,
    /// Max query depth (prevents abuse)
    #[serde(default = "default_depth")]
    pub max_depth: usize,
    /// Max query complexity
    #[serde(default = "default_complexity")]
    pub max_complexity: usize,
    /// Enable introspection (disable in production if needed)
    #[serde(default = "default_true")]
    pub introspection: bool,
    /// Default page size for list queries
    #[serde(default = "default_page")]
    pub default_page_size: usize,
    /// Max page size
    #[serde(default = "default_max_page")]
    pub max_page_size: usize,
}

fn default_true() -> bool { true }
fn default_depth() -> usize { 10 }
fn default_complexity() -> usize { 1000 }
fn default_page() -> usize { 20 }
fn default_max_page() -> usize { 100 }

impl Default for GraphQLConfig {
    fn default() -> Self {
        Self {
            enabled: true, playground: true,
            max_depth: 10, max_complexity: 1000,
            introspection: true,
            default_page_size: 20, max_page_size: 100,
        }
    }
}
