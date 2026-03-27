# Hello Server

Complete minimal VilApp hello server with ShmSlice and ServiceCtx.

## Full Example

```rust
use vil_server::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct GreetResponse {
    message: String,
    server: &'static str,
}

#[tokio::main]
async fn main() {
    let service = ServiceProcess::new("hello")
        .endpoint(Method::GET, "/", get(hello))
        .endpoint(Method::GET, "/greet/:name", get(greet))
        .endpoint(Method::POST, "/echo", post(echo));

    VilApp::new("hello-server")
        .port(8080)
        .service(service)
        .run()
        .await;
}

async fn hello() -> &'static str {
    "Hello from VIL!"
}

async fn greet(Path(name): Path<String>) -> VilResponse<GreetResponse> {
    VilResponse::ok(GreetResponse {
        message: format!("Hello, {}!", name),
        server: "vil-server",
    })
}

#[vil_handler(shm)]
async fn echo(ctx: ServiceCtx, slice: ShmSlice) -> VilResponse<serde_json::Value> {
    let body: serde_json::Value = vil_json::from_slice(slice.as_ref())?;
    VilResponse::ok(body)
}
```

## What Each Type Does

| Type | Role |
|------|------|
| `ServiceProcess::new("hello")` | Creates a named service process |
| `.endpoint(Method::GET, "/", get(handler))` | Registers an HTTP endpoint |
| `VilApp::new("hello-server")` | Creates the application container |
| `.port(8080)` | Sets the listen port |
| `.service(service)` | Attaches a service process |
| `.run().await` | Starts the server |
| `ShmSlice` | Zero-copy request body from SHM |
| `ServiceCtx` | Typed state + Tri-Lane metadata |
| `VilResponse::ok(data)` | 200 OK with SIMD JSON body |

## Cargo.toml

```toml
[dependencies]
vil_server = { path = "../../crates/vil_server" }
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
```

## Run

```bash
cargo run
# GET  http://localhost:8080/
# GET  http://localhost:8080/greet/world
# POST http://localhost:8080/echo  -d '{"key":"value"}'
```

> Reference: docs/vil/003-VIL-Developer_Guide-Server-Framework.md
