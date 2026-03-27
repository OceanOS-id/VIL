# VilPlugin Trait

The `VilPlugin` trait defines how plugins register capabilities, routes, and state into a VilApp.

## Trait Definition

```rust
pub trait VilPlugin: Send + Sync + 'static {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn register(&self, ctx: &mut PluginContext) -> Result<(), PluginError>;
}
```

## Implementing a Plugin

```rust
use vil_server::plugin::{VilPlugin, PluginContext, PluginError};

struct MetricsPlugin {
    retention_hours: u64,
}

impl VilPlugin for MetricsPlugin {
    fn id(&self) -> &str { "vil.metrics" }
    fn name(&self) -> &str { "Metrics Collector" }

    fn register(&self, ctx: &mut PluginContext) -> Result<(), PluginError> {
        // Register state
        ctx.state(MetricsStore::new(self.retention_hours));

        // Register endpoints
        ctx.endpoint(Method::GET, "/metrics", get(metrics_handler));
        ctx.endpoint(Method::GET, "/metrics/reset", post(reset_handler));

        // Register middleware
        ctx.middleware(MetricsMiddleware::new());

        Ok(())
    }
}
```

## PluginContext API

| Method | Description |
|--------|-------------|
| `ctx.state(T)` | Inject typed shared state |
| `ctx.endpoint(method, path, handler)` | Register HTTP endpoint |
| `ctx.middleware(M)` | Add middleware layer |
| `ctx.config::<T>()` | Read typed configuration |
| `ctx.service_name()` | Parent service name |

## Registering Plugins

```rust
VilApp::new("my-app")
    .port(8080)
    .plugin(MetricsPlugin { retention_hours: 24 })
    .plugin(LlmPlugin::new().provider(openai))
    .plugin(RagPlugin::new().vector_store(config))
    .service(api_service)
    .run()
    .await;
```

## Built-in Plugins

| Plugin | Crate | Description |
|--------|-------|-------------|
| `LlmPlugin` | `vil_llm` | LLM chat + streaming |
| `RagPlugin` | `vil_rag` | Document retrieval |
| `AgentPlugin` | `vil_agent` | Tool-augmented agents |
| `EmbedderPlugin` | `vil_embedder` | Text embeddings |
| `GuardrailsPlugin` | `vil_guardrails` | Content safety |
| `TokenizerPlugin` | `vil_tokenizer` | Token counting |
| `VectorDbPlugin` | `vil_vectordb` | Vector storage |

## Plugin Lifecycle

1. `VilApp::new()` collects plugins
2. During `.run()`, each plugin's `register()` is called in order
3. Plugin state is accessible via `ServiceCtx::state::<T>()`
4. Plugins share the same ExchangeHeap and Tri-Lane mesh

## Error Handling

```rust
fn register(&self, ctx: &mut PluginContext) -> Result<(), PluginError> {
    let api_key = ctx.config::<String>("llm.api_key")
        .ok_or(PluginError::config("Missing llm.api_key"))?;
    // ...
    Ok(())
}
```

> Reference: docs/vil/005-VIL-Developer_Guide-Plugins-AI.md
