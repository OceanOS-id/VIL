// 303 — RAG Hybrid Exact+Semantic (Python WASM)
#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/303-rag-hybrid-exact-semantic/vwfd/workflows", 3112)
        .sidecar("rag_exact_match_check", "python3 -u examples/303-rag-hybrid-exact-semantic/vwfd/sidecar/python/rag_hybrid_search.py")
        .sidecar("rag_keyword_score", "python3 -u examples/303-rag-hybrid-exact-semantic/vwfd/sidecar/python/rag_hybrid_search.py")
        .run().await;
}
