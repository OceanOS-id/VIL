# RAG Plugin

`vil_rag` provides document ingestion, vector search, and retrieval-augmented generation.

## Basic RAG Pipeline

```rust
use vil_rag::prelude::*;

let rag = RagPipeline::new()
    .embedder(EmbedderConfig::openai("text-embedding-3-small"))
    .vector_store(VectorStoreConfig::in_memory())
    .llm(LlmProvider::openai().model("gpt-4o").build())
    .build();

// Ingest documents
rag.ingest(vec![
    Document::text("VIL uses ShmSlice for zero-copy body extraction."),
    Document::text("ServiceCtx provides typed state access."),
]).await?;

// Query with retrieval
let answer = rag.query("How does VIL handle request bodies?").await?;
println!("{}", answer.content);
```

## Document Store

```rust
// From text
let doc = Document::text("plain text content");

// From file
let doc = Document::file("path/to/document.pdf")?;

// With metadata
let doc = Document::text("content")
    .metadata("source", "api-docs")
    .metadata("version", "3.0");

// Bulk ingest with chunking
rag.ingest_with_config(documents, ChunkConfig {
    strategy: ChunkStrategy::Sliding,
    chunk_size: 512,
    overlap: 64,
}).await?;
```

## Vector Search

```rust
// Direct similarity search (without LLM)
let results = rag.search("zero-copy transport", 5).await?;
for result in results {
    println!("Score: {:.3} | {}", result.score, result.content);
}
```

## As VilPlugin

```rust
use vil_rag::RagPlugin;

VilApp::new("rag-service")
    .port(8080)
    .plugin(RagPlugin::new()
        .embedder(embedder_config)
        .vector_store(vector_config))
    .service(api_service)
    .run()
    .await;
```

## Server Handler

```rust
#[vil_handler(shm)]
async fn query_rag(ctx: ServiceCtx, slice: ShmSlice) -> VilResponse<RagAnswer> {
    let input: RagQuery = slice.json()?;
    let rag = ctx.state::<RagPipeline>();
    let answer = rag.query(&input.question).await?;
    VilResponse::ok(answer)
}
```

## Chunk Strategies

| Strategy | Description |
|----------|-------------|
| `Sliding` | Fixed-size windows with overlap |
| `Sentence` | Split on sentence boundaries |
| `Paragraph` | Split on paragraph boundaries |
| `Code` | Language-aware code splitting |
| `Table` | Preserve table structure |

> Reference: docs/vil/005-VIL-Developer_Guide-Plugins-AI.md
