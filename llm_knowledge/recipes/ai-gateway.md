# Recipe: AI Gateway

SSE streaming gateway to AI providers with .transform() and ShmToken.

## Full Example

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
        .header("Authorization", "Bearer ${ENV:OPENAI_API_KEY}")
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

## With Content Filtering

Add `.transform()` to filter or modify AI responses:

```rust
let source = HttpSourceBuilder::new()
    .url("https://api.openai.com/v1/chat/completions")
    .format(HttpFormat::SSE)
    .dialect(SseSourceDialect::OpenAi)
    .json_tap("choices[0].delta.content")
    .transform(|chunk: &[u8]| -> Option<Vec<u8>> {
        let text = std::str::from_utf8(chunk).ok()?;
        // Block responses containing sensitive terms
        if text.contains("CONFIDENTIAL") {
            None
        } else {
            Some(chunk.to_vec())
        }
    })
    .build();
```

## Multi-Provider Gateway

Route to different providers based on client request:

```rust
let openai_source = HttpSourceBuilder::new()
    .url("https://api.openai.com/v1/chat/completions")
    .format(HttpFormat::SSE)
    .dialect(SseSourceDialect::OpenAi)
    .json_tap("choices[0].delta.content")
    .build();

let anthropic_source = HttpSourceBuilder::new()
    .url("https://api.anthropic.com/v1/messages")
    .format(HttpFormat::SSE)
    .dialect(SseSourceDialect::Anthropic)
    .json_tap("delta.text")
    .build();

let ollama_source = HttpSourceBuilder::new()
    .url("http://localhost:11434/api/chat")
    .format(HttpFormat::SSE)
    .dialect(SseSourceDialect::Ollama)
    .json_tap("message.content")
    .build();
```

## Test

```bash
# Trigger AI chat via gateway
curl -X POST http://localhost:3080/chat \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "Hello!"}],
    "stream": true
  }'

# Response streams back as SSE
data: Hello
data: !
data:  How
data:  can
data:  I
data:  help?
data: [DONE]
```

## VilApp Alternative

Same gateway as a VilApp server handler:

```rust
#[vil_handler(shm)]
async fn chat(ctx: ServiceCtx, slice: ShmSlice) -> impl IntoResponse {
    let llm = ctx.state::<LlmProvider>();
    let input: ChatRequest = slice.json()?;
    let stream = llm.chat_stream(input.messages).await?;
    SseStream::new(stream)
}
```

> Reference: docs/vil/004-VIL-Developer_Guide-Pipeline-Streaming.md
