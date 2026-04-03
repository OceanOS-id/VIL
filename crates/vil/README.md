# VIL — Pythonic Rust Framework

Zero-copy, high-performance backend framework with batteries included.

```toml
[dependencies]
vil = { version = "0.2", features = ["web", "db-sqlite"] }
```

```rust
use vil::prelude::*;

#[vil_handler]
async fn hello() -> VilResponse<&'static str> {
    VilResponse::ok("Hello VIL!")
}

#[tokio::main]
async fn main() {
    let svc = ServiceProcess::new("hello")
        .endpoint(Method::GET, "/", get(hello));
    VilApp::new("my-app")
        .port(8082)
        .observer(true)
        .service(svc)
        .run().await;
}
```

## Features

| Feature | Includes |
|---------|----------|
| `web` (default) | Server, auth, macros, JSON |
| `log` (default) | Semantic logging (7 types) |
| `db-sqlite` | SQLite via sqlx |
| `db-postgres` | PostgreSQL via sqlx |
| `ai` | LLM, gateway, prompts, guardrails |
| `full` | Everything |

[Documentation](https://vastar.id/docs/vil) | [GitHub](https://github.com/OceanOS-id/VIL)
