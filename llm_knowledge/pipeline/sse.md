# SSE Pipeline

Server-Sent Events streaming with dialect support for AI provider APIs.

## Basic SSE Pipeline

```rust
use vil_sdk::prelude::*;
use vil_sdk::http::{HttpSinkBuilder, HttpSourceBuilder, HttpFormat, SseSourceDialect};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sink = HttpSinkBuilder::new()
        .port(3080)
        .path("/chat")
        .build();

    let source = HttpSourceBuilder::new()
        .url("https://api.openai.com/v1/chat/completions")
        .format(HttpFormat::SSE)
        .dialect(SseSourceDialect::OpenAi)
        .json_tap("choices[0].delta.content")
        .build();

    let (_ir, handles) = vil_workflow! {
        name: "AiGateway",
        token: ShmToken,
        instances: [ sink, source ],
        routes: [
            sink.out -> source.in (LoanWrite),
            source.data -> sink.in (LoanWrite),
        ]
    };

    for h in handles { h.join().unwrap(); }
    Ok(())
}
```

## SSE Dialects

Each AI provider uses a different SSE envelope. Dialects handle parsing automatically.

| Dialect | Provider | Content Path |
|---------|----------|--------------|
| `SseSourceDialect::OpenAi` | OpenAI, Azure OpenAI | `choices[0].delta.content` |
| `SseSourceDialect::Anthropic` | Anthropic Claude | `delta.text` |
| `SseSourceDialect::Ollama` | Ollama (local) | `message.content` |
| `SseSourceDialect::Standard` | Generic SSE | Raw `data:` field |

## json_tap

Extracts a nested field from SSE JSON events, forwarding only that value:

```rust
// OpenAI: extract content text from nested JSON
.json_tap("choices[0].delta.content")

// Anthropic: extract text delta
.json_tap("delta.text")

// Without json_tap: forwards entire SSE data payload
```

## Dialect Examples

```rust
// OpenAI
let source = HttpSourceBuilder::new()
    .url("https://api.openai.com/v1/chat/completions")
    .format(HttpFormat::SSE)
    .dialect(SseSourceDialect::OpenAi)
    .json_tap("choices[0].delta.content")
    .build();

// Anthropic
let source = HttpSourceBuilder::new()
    .url("https://api.anthropic.com/v1/messages")
    .format(HttpFormat::SSE)
    .dialect(SseSourceDialect::Anthropic)
    .json_tap("delta.text")
    .build();

// Ollama (local)
let source = HttpSourceBuilder::new()
    .url("http://localhost:11434/api/chat")
    .format(HttpFormat::SSE)
    .dialect(SseSourceDialect::Ollama)
    .json_tap("message.content")
    .build();
```

## SSE with .transform()

Filter or modify SSE events inline:

```rust
let source = HttpSourceBuilder::new()
    .url("https://api.openai.com/v1/chat/completions")
    .format(HttpFormat::SSE)
    .dialect(SseSourceDialect::OpenAi)
    .json_tap("choices[0].delta.content")
    .transform(|chunk: &[u8]| -> Option<Vec<u8>> {
        let text = std::str::from_utf8(chunk).ok()?;
        // Redact sensitive content
        let cleaned = text.replace("SECRET", "[REDACTED]");
        Some(cleaned.into_bytes())
    })
    .build();
```

## SSE Format

```
data: {"choices":[{"delta":{"content":"Hello"}}]}\n\n
data: {"choices":[{"delta":{"content":" world"}}]}\n\n
data: [DONE]\n\n
```

> Reference: docs/vil/004-VIL-Developer_Guide-Pipeline-Streaming.md
