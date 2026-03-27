/// Unified DB error — stack-allocated enum.
#[derive(Debug)]
pub enum DbError {
    NotFound,
    ConnectionFailed(String),
    QueryFailed(String),
    CapabilityMissing(String),
    SchemaValidationFailed(String),
    ProviderError(String),
    Timeout,
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "Entity not found"),
            Self::ConnectionFailed(e) => write!(f, "Connection failed: {}", e),
            Self::QueryFailed(e) => write!(f, "Query failed: {}", e),
            Self::CapabilityMissing(c) => write!(f, "Capability missing: {}", c),
            Self::SchemaValidationFailed(e) => write!(f, "Schema validation: {}", e),
            Self::ProviderError(e) => write!(f, "Provider: {}", e),
            Self::Timeout => write!(f, "Query timeout"),
        }
    }
}

impl std::error::Error for DbError {}

pub type DbResult<T> = Result<T, DbError>;
