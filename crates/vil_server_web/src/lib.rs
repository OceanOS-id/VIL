// =============================================================================
// VIL Server Web — Handler utilities, validation, error handling
// =============================================================================

pub mod validation;
pub mod error;
pub mod openapi;

pub use validation::Valid;
pub use error::HandlerError;
pub use error::HandlerResult;

// Re-export core types for handler convenience
pub use vil_server_core::Json;
pub use vil_server_core::Path;
pub use vil_server_core::Query;
pub use vil_server_core::State;
pub use vil_server_core::VilError;
pub use vil_server_core::RequestId;
pub use vil_server_core::StatusCode;
pub use vil_server_core::IntoResponse;
