// =============================================================================
// VIL gRPC — tonic integration for pipeline and server
// =============================================================================
//
// Provides:
//   - GrpcGatewayBuilder: 5-line gRPC server (vil_sdk level)
//   - gRPC health check (grpc.health.v1)
//   - Per-service metrics
//   - VilServer dual-port support (HTTP + gRPC)

pub mod gateway;
pub mod health;
pub mod metrics;
pub mod config;

pub use gateway::GrpcGatewayBuilder;
pub use config::GrpcServerConfig;
