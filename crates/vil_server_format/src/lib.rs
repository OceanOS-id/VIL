// =============================================================================
// VIL Server Format — Multi-Format Response
// =============================================================================
//
// FormatResponse<T> auto-negotiates response format based on Accept header.
// Community: JSON + Protobuf. Commercial: + MessagePack + FlatBuffers.

pub mod format_response;
pub mod negotiator;

pub use format_response::FormatResponse;
pub use negotiator::ResponseFormat;
