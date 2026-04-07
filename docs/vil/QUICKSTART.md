# VIL QUICKSTART — Build Your First API in 30 Minutes

> From zero to running backend with auth, database, and AI — in 6 steps.

---

## Part 1: Create Project (2 min)

```bash
# Install VIL CLI (one time)
cargo install vil-cli

# Create project
vil init my-app --template rest-api
cd my-app

# Run
cargo run
```

Visit:
- http://localhost:8082/health → `{"status":"healthy"}`
- http://localhost:8082/_vil/dashboard/ → Observer Dashboard

---

## Part 2: Your First Endpoint (5 min)

Edit `src/services/hello.rs`:

```rust
use vil::prelude::*;

#[vil_handler]
pub async fn index() -> VilResponse<&'static str> {
    VilResponse::ok("Hello VIL!")
}

#[vil_handler]
pub async fn greet(Path(name): Path<String>) -> VilResponse<String> {
    VilResponse::ok(format!("Hello {}!", name))
}
```

Register in `src/main.rs`:

```rust
let hello = ServiceProcess::new("hello")
    .endpoint(Method::GET, "/", get(services::hello::index))
    .endpoint(Method::GET, "/:name", get(services::hello::greet));

VilApp::new("my-app")
    .port(8082)
    .observer(true)
    .service(hello)
    .run().await;
```

Test:
```bash
curl localhost:8082/api/hello/
# → "Hello VIL!"

curl localhost:8082/api/hello/World
# → "Hello World!"
```

---

## Part 3: Database + Model (5 min)

Create migration:
```bash
mkdir -p migrations
cat > migrations/001_create_todos.up.sql << 'SQL'
CREATE TABLE todos (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    done INTEGER DEFAULT 0,
    created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
SQL
```

Define model in `src/models/todo.rs`:

```rust
use serde::{Deserialize, Serialize};
use vil::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize, VilModel, sqlx::FromRow)]
pub struct Todo {
    pub id: String,
    pub title: String,
    pub done: i64,
    pub created_at: String,
}

#[derive(Deserialize)]
pub struct CreateTodo {
    pub title: String,
}
```

Create handler in `src/services/todos.rs`:

```rust
use vil::prelude::*;
use crate::models::todo::*;

#[vil_handler]
pub async fn list(ctx: ServiceCtx) -> Result<VilResponse<Vec<Todo>>, VilError> {
    let pool = ctx.state::<AppState>()?.pool.inner();
    let todos = sqlx::query_as::<_, Todo>("SELECT * FROM todos ORDER BY created_at DESC")
        .fetch_all(pool).await.map_err(|e| VilError::internal(e.to_string()))?;
    Ok(VilResponse::ok(todos))
}

#[vil_handler]
pub async fn create(ctx: ServiceCtx, body: ShmSlice) -> Result<VilResponse<Todo>, VilError> {
    let pool = ctx.state::<AppState>()?.pool.inner();
    let req: CreateTodo = body.json().map_err(|_| VilError::bad_request("Invalid JSON"))?;
    let id = uuid::Uuid::new_v4().to_string();

    sqlx::query("INSERT INTO todos (id, title) VALUES (?, ?)")
        .bind(&id).bind(&req.title)
        .execute(pool).await.map_err(|e| VilError::internal(e.to_string()))?;

    let todo = sqlx::query_as::<_, Todo>("SELECT * FROM todos WHERE id = ?")
        .bind(&id).fetch_one(pool).await.map_err(|e| VilError::internal(e.to_string()))?;

    Ok(VilResponse::created(todo))
}
```

Register:
```rust
let todos = ServiceProcess::new("todos")
    .endpoint(Method::GET, "/", get(services::todos::list))
    .endpoint(Method::POST, "/", post(services::todos::create))
    .state(state.clone());
```

Test:
```bash
curl -X POST localhost:8082/api/todos/ \
  -H 'Content-Type: application/json' \
  -d '{"title":"Learn VIL"}'
# → {"id":"...","title":"Learn VIL","done":0,...}

curl localhost:8082/api/todos/
# → [{"id":"...","title":"Learn VIL",...}]
```

---

## Part 4: Authentication (5 min)

VIL includes password hashing + JWT out of the box:

```rust
use vil::prelude::*;

// Hash password on register
let hash = VilPassword::hash("mypassword")?;

// Verify on login
let valid = VilPassword::verify("mypassword", &hash)?;

// Sign JWT tokens
let jwt = VilJwt::new("your-secret-key")
    .access_expiry(Duration::from_secs(900));    // 15 min
let pair = jwt.sign_pair(&MyClaims { sub: user_id, role: "user" })?;
// pair.access_token, pair.refresh_token

// Protected endpoint — auto-extract claims from header
#[vil_handler]
pub async fn profile(VilClaims(claims): VilClaims<MyClaims>) -> VilResult<Profile> {
    // claims.sub = authenticated user ID
    let profile = fetch_profile(&claims.sub).await?;
    Ok(VilResponse::ok(profile))
}
```

Client sends: `Authorization: Bearer <access_token>`

---

## Part 5: AI Integration (5 min)

VIL has built-in LLM provider support:

```rust
use vil_llm::{OpenAiConfig, OpenAiProvider, LlmProvider, ChatMessage};

// Connect to Groq (OpenAI-compatible)
let provider = OpenAiProvider::new(
    OpenAiConfig::new("your-groq-key", "llama-3.3-70b-versatile")
        .base_url("https://api.groq.com/openai/v1")
);

// Chat
let response = provider.chat(&[
    ChatMessage::system("You are a helpful assistant."),
    ChatMessage::user("Explain TOEFL in one sentence."),
]).await?;

println!("{}", response.content);
// → "TOEFL is a standardized test measuring English proficiency..."
```

Works with: **Groq, OpenAI, Anthropic, Ollama** — same API.

---

## Part 6: Deploy (5 min)

```bash
# Build optimized binary
cargo build --release
ls -lh target/release/my-app
# → ~15 MB single binary

# Copy to server
scp target/release/my-app server:/opt/my-app/

# Run with systemd
ssh server "systemctl restart my-app"

# Verify
curl https://api.my-app.com/health
# → {"status":"healthy"}
```

Monitor at: `https://api.my-app.com/_vil/dashboard/`

---

## What You Get For Free

| Feature | How | Cost |
|---------|-----|------|
| Health check | `GET /health` | Zero config |
| Readiness probe | `GET /ready` | Zero config |
| Prometheus metrics | `GET /metrics` | Zero config |
| Server info | `GET /info` | Zero config |
| Live dashboard | `/_vil/dashboard/` | `.observer(true)` |
| Request tracing | `#[vil_handler]` | Zero code |
| Access logging | `#[vil_handler]` | Zero code |
| Zero-copy bodies | `ShmSlice` | Automatic |
| 41,000 req/s | VIL runtime | Built-in |

---

## Next Steps

- **[Developer Guide Part 1](001-VIL-Developer_Guide-Overview.md)** — Architecture deep dive
- **[Developer Guide Part 3](003-VIL-Developer_Guide-Server-Framework.md)** — Server patterns
- **[Developer Guide Part 11](011-VIL-Developer_Guide-Custom-Code.md)** — WASM & Sidecar macros
- **[Examples](../../examples/)** — 112 working examples
- **[COOKBOOK](COOKBOOK.md)** — 20 common patterns (coming soon)

---

## One-liner Summary

```toml
# Cargo.toml — that's it
[dependencies]
vil = { version = "0.2", features = ["web", "db-sqlite"] }
```

```rust
// main.rs — 10 lines to production
use vil::prelude::*;

#[vil_handler]
async fn hello() -> VilResponse<&'static str> { VilResponse::ok("Hello!") }

#[tokio::main]
async fn main() {
    let svc = ServiceProcess::new("hello")
        .endpoint(Method::GET, "/", get(hello));
    VilApp::new("my-app").port(8082).observer(true).service(svc).run().await;
}
```
