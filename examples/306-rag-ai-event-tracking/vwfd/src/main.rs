// 306 — RAG AI Event Tracking (Python WASM)
#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/306-rag-ai-event-tracking/vwfd/workflows", 8080)
        .wasm("rag_keyword_article_search", "examples/306-rag-ai-event-tracking/vwfd/wasm/python/rag_keyword_search.py")
        .run().await;
}
