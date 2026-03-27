# gRPC Integration

`vil_grpc` provides gRPC gateway, health checks, and protobuf support.

## Setup

```rust
use vil_grpc::prelude::*;

let grpc = GrpcGateway::new()
    .add_service(UserServiceServer::new(UserServiceImpl))
    .add_service(OrderServiceServer::new(OrderServiceImpl))
    .health_check(true)
    .reflection(true)
    .build();

VilApp::new("grpc-service")
    .port(8080)          // HTTP + GraphQL
    .grpc_port(50051)    // gRPC
    .grpc(grpc)
    .run()
    .await;
```

## Service Implementation

```rust
pub struct UserServiceImpl;

#[tonic::async_trait]
impl UserService for UserServiceImpl {
    async fn get_user(&self, request: Request<GetUserRequest>) -> Result<Response<User>, Status> {
        let req = request.into_inner();
        let user = find_user(req.id).await
            .map_err(|_| Status::not_found("User not found"))?;
        Ok(Response::new(user))
    }
}
```

## Health Check

Auto-registered gRPC health service:

```bash
grpcurl -plaintext localhost:50051 grpc.health.v1.Health/Check
```

## Metrics

| Metric | Description |
|--------|-------------|
| `grpc_requests_total` | Total gRPC requests |
| `grpc_request_duration_seconds` | Request latency histogram |
| `grpc_active_connections` | Active connection count |

## Proto Definition

```protobuf
service UserService {
  rpc GetUser (GetUserRequest) returns (User);
  rpc ListUsers (ListUsersRequest) returns (stream User);
}
```

> Reference: docs/vil/006-VIL-Developer_Guide-CLI-Deployment.md
