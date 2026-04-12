// 302 — RAG Multi-Source Fan-In (Python WASM)
#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/302-rag-multi-source-fanin/vwfd/workflows", 3111)
        .wasm("rag_multi_source_search", "examples/302-rag-multi-source-fanin/vwfd/wasm/python/rag_multi_source.py")
        .wasm("rag_cross_rank_merge", "examples/302-rag-multi-source-fanin/vwfd/wasm/python/rag_multi_source.py")
        .run().await;
}
