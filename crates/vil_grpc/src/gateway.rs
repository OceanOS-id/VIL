// =============================================================================
// VIL gRPC — Gateway Builder (5-line API)
// =============================================================================
//
// Usage:
//   vil_grpc::GrpcGatewayBuilder::new()
//       .listen(50051)
//       .health_check(true)
//       .run()
//       .await;

use crate::config::GrpcServerConfig;
use std::net::SocketAddr;
use tonic::transport::Server;

/// Builder for a gRPC gateway server.
///
/// Wraps tonic::Server with VIL conventions:
/// auto health check, metrics, structured logging.
pub struct GrpcGatewayBuilder {
    config: GrpcServerConfig,
}

impl GrpcGatewayBuilder {
    pub fn new() -> Self {
        Self {
            config: GrpcServerConfig::default(),
        }
    }

    /// Set the gRPC listen port (default: 50051).
    pub fn listen(mut self, port: u16) -> Self {
        self.config.port = port;
        self
    }

    /// Enable/disable gRPC health check service.
    pub fn health_check(mut self, enabled: bool) -> Self {
        self.config.health_check = enabled;
        self
    }

    /// Enable/disable gRPC server reflection.
    pub fn reflection(mut self, enabled: bool) -> Self {
        self.config.reflection = enabled;
        self
    }

    /// Set max message size in bytes.
    pub fn max_message_size(mut self, bytes: usize) -> Self {
        self.config.max_message_size = bytes;
        self
    }

    /// Get the config.
    pub fn config(&self) -> &GrpcServerConfig {
        &self.config
    }

    /// Get the socket address.
    pub fn addr(&self) -> SocketAddr {
        SocketAddr::from(([0, 0, 0, 0], self.config.port))
    }

    /// Build a tonic Server with the configured settings.
    ///
    /// Users add their services to the returned builder:
    /// ```ignore
    /// let builder = gateway.build();
    /// let server = builder.add_service(MyServiceServer::new(impl));
    /// server.serve(gateway.addr()).await?;
    /// ```
    pub fn build(&self) -> Server {
        let mut server = Server::builder();

        if self.config.max_concurrent_streams > 0 {
            server = server
                .concurrency_limit_per_connection(self.config.max_concurrent_streams as usize);
        }

        server
    }
}

impl Default for GrpcGatewayBuilder {
    fn default() -> Self {
        Self::new()
    }
}
