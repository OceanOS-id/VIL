// 302 — RAG Multi-Source Fan-In (Python WASM)
#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/302-rag-multi-source-fanin/vwfd/workflows", 3111)
        .sidecar("rag_multi_source_search", "python3 -u examples/302-rag-multi-source-fanin/vwfd/sidecar/python/rag_multi_source.py")
        .sidecar("rag_cross_rank_merge", "python3 -u examples/302-rag-multi-source-fanin/vwfd/sidecar/python/rag_multi_source.py")
        .run().await;
}
