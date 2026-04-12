// 303 — RAG Hybrid Exact+Semantic (Python WASM)
#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/303-rag-hybrid-exact-semantic/vwfd/workflows", 3112)
        .wasm("rag_exact_match_check", "examples/303-rag-hybrid-exact-semantic/vwfd/wasm/python/rag_hybrid_search.py")
        .wasm("rag_keyword_score", "examples/303-rag-hybrid-exact-semantic/vwfd/wasm/python/rag_hybrid_search.py")
        .run().await;
}
