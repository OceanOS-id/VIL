// =============================================================================
// VIL Server Handler Error — Convenience error conversions
// =============================================================================

use vil_server_core::error::VilError;

/// Convenience alias for handler return types.
pub type HandlerError = VilError;

/// Convenience Result type for handlers.
pub type HandlerResult<T> = Result<T, HandlerError>;
