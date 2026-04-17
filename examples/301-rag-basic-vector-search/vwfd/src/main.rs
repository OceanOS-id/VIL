// 301 — RAG Basic Vector Search (Python WASM)
#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/301-rag-basic-vector-search/vwfd/workflows", 3110)
        .sidecar("rag_embed_and_search", "python3 -u examples/301-rag-basic-vector-search/vwfd/sidecar/python/rag_embed_and_search.py")
        .run().await;
}
