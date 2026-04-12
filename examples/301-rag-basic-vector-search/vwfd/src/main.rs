// 301 — RAG Basic Vector Search (Python WASM)
#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/301-rag-basic-vector-search/vwfd/workflows", 3110)
        .wasm("rag_embed_and_search", "examples/301-rag-basic-vector-search/vwfd/wasm/python/rag_embed_and_search.py")
        .run().await;
}
