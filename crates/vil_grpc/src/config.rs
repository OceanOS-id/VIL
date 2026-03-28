use serde::{Deserialize, Serialize};

/// gRPC server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcServerConfig {
    /// Listen port
    #[serde(default = "default_port")]
    pub port: u16,
    /// Max message size (bytes)
    #[serde(default = "default_max_msg")]
    pub max_message_size: usize,
    /// Enable gRPC health check service
    #[serde(default = "default_true")]
    pub health_check: bool,
    /// Enable gRPC server reflection
    #[serde(default = "default_true")]
    pub reflection: bool,
    /// Max concurrent streams per connection
    #[serde(default = "default_streams")]
    pub max_concurrent_streams: u32,
}

fn default_port() -> u16 {
    50051
}
fn default_max_msg() -> usize {
    4 * 1024 * 1024
}
fn default_true() -> bool {
    true
}
fn default_streams() -> u32 {
    200
}

impl Default for GrpcServerConfig {
    fn default() -> Self {
        Self {
            port: 50051,
            max_message_size: 4 * 1024 * 1024,
            health_check: true,
            reflection: true,
            max_concurrent_streams: 200,
        }
    }
}
