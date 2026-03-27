# LLM Plugin

`vil_llm` provides multi-provider LLM chat, streaming, and model routing.

## Basic Chat

```rust
use vil_llm::prelude::*;

let provider = LlmProvider::openai()
    .api_key(std::env::var("OPENAI_API_KEY")?)
    .model("gpt-4o")
    .build();

let response = provider.chat(vec![
    Message::system("You are a helpful assistant."),
    Message::user("Explain zero-copy in one sentence."),
]).await?;

println!("{}", response.content);
```

## Streaming Chat

```rust
let mut stream = provider.chat_stream(vec![
    Message::user("Write a haiku about Rust."),
]).await?;

while let Some(chunk) = stream.next().await {
    print!("{}", chunk?.delta);
}
```

## Multi-Model Routing

Route requests to different models based on criteria:

```rust
use vil_llm::router::LlmRouter;

let router = LlmRouter::new()
    .route("fast", LlmProvider::openai().model("gpt-4o-mini").build())
    .route("smart", LlmProvider::openai().model("gpt-4o").build())
    .route("local", LlmProvider::ollama().model("llama3").build())
    .default("fast");

// Route by name
let response = router.chat("smart", messages).await?;

// Auto-route by token count
let response = router.auto_route(messages).await?;
```

## Supported Providers

| Provider | Constructor | Models |
|----------|------------|--------|
| OpenAI | `LlmProvider::openai()` | gpt-4o, gpt-4o-mini |
| Anthropic | `LlmProvider::anthropic()` | claude-3.5-sonnet |
| Ollama | `LlmProvider::ollama()` | llama3, mistral, codellama |

## As VilPlugin

```rust
use vil_llm::LlmPlugin;

VilApp::new("ai-service")
    .port(8080)
    .plugin(LlmPlugin::new()
        .provider(openai_provider)
        .provider(anthropic_provider))
    .service(api_service)
    .run()
    .await;
```

## Pipeline Integration

Use LLM in a streaming pipeline with SSE:

```rust
let source = HttpSourceBuilder::new()
    .url("https://api.openai.com/v1/chat/completions")
    .format(HttpFormat::SSE)
    .dialect(SseSourceDialect::OpenAi)
    .json_tap("choices[0].delta.content")
    .build();
```

> Reference: docs/vil/005-VIL-Developer_Guide-Plugins-AI.md
