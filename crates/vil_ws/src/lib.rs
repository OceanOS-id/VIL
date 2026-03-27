// =============================================================================
// vil_ws — VIL Dedicated WebSocket Server
// =============================================================================
//
// WebSocket server with room/channel management:
//   - mq_log! auto-emit on every send (op_type=0) and receive (op_type=1)
//   - VIL-compliant plain enum error type (WsFault)
//   - No println!, tracing::info!, or log::info! — COMPLIANCE.md §8
//
// Thread hint: WsServer spawns 1 accept loop task per server instance.
// Add 1 to your LogConfig.threads for optimal log ring sizing.
// =============================================================================

pub mod config;
pub mod server;
pub mod room;
pub mod error;
pub mod process;

pub use config::WsConfig;
pub use server::WsServer;
pub use room::RoomManager;
pub use error::WsFault;
