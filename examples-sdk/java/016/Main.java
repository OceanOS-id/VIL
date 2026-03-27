// 016 — AI RAG Gateway (SSE Pipeline)
// Equivalent to: examples/016-basic-ai-rag-gateway (Rust)
// Compile: vil compile --from java --input 016/Main.java --release
package dev.vil.examples;

import dev.vil.VilPipeline;
import java.util.Map;
import java.util.List;

public class Main {
    public static void main(String[] args) {
        VilPipeline pipeline = new VilPipeline("ai-rag-gateway", 3084);

        // -- Semantic types ---------------------------------------------------
        pipeline.semanticType("RagState", "state", Map.of(
            "query_id", "u64",
            "chunks_retrieved", "u32",
            "tokens_generated", "u32"
        ));
        pipeline.fault("RagFault", List.of(
            "VectorDbTimeout", "EmbeddingError", "UpstreamTimeout", "ParseError"
        ));

        // -- Nodes ------------------------------------------------------------
        pipeline.sink(3084, "/rag", "gateway");
        pipeline.source("http://localhost:4545/v1/chat/completions",
            "sse", "openai", "choices[0].delta.content", "inference");

        // -- Transform: inject retrieved context before LLM call --------------
        pipeline.transform("context_inject", """
            let mut req = serde_json::from_slice(line).ok()?;
            let ctx = retrieve_context(&req["query"].as_str()?);
            req["messages"][0]["content"] = serde_json::json!(ctx);
            Some(serde_json::to_vec(&req).ok()?)
            """);

        // -- Tri-Lane routes --------------------------------------------------
        pipeline.route("gateway.trigger_out", "context_inject.trigger_in", "LoanWrite");
        pipeline.route("context_inject.data_out", "inference.trigger_in", "LoanWrite");
        pipeline.route("inference.data_out", "gateway.data_in", "LoanWrite");
        pipeline.route("inference.ctrl_out", "gateway.ctrl_in", "Copy");

        // -- Emit / compile ---------------------------------------------------
        if ("manifest".equals(System.getenv("VIL_COMPILE_MODE"))) {
            System.out.println(pipeline.toYaml());
        } else {
            pipeline.compile();
        }
    }
}
