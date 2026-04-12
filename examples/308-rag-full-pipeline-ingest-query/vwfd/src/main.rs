// 308 — RAG Full Pipeline (Python WASM)
#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/308-rag-full-pipeline-ingest-query/vwfd/workflows", 8080)
        .wasm("rag_hnsw_embed_search", "examples/308-rag-full-pipeline-ingest-query/vwfd/wasm/python/rag_embed_search.py")
        .run().await;
}
