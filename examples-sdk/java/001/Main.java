// 001 — AI Gateway (SSE Pipeline)
// Equivalent to: examples/001-basic-ai-gw-demo (Rust)
// Compile: vil compile --from java --input 001/Main.java --release
package dev.vil.examples;

import dev.vil.VilPipeline;
import java.util.Map;
import java.util.List;

public class Main {
    public static void main(String[] args) {
        VilPipeline pipeline = new VilPipeline("ai-gateway", 3080);

        // -- Semantic types ---------------------------------------------------
        pipeline.semanticType("InferenceState", "state", Map.of(
            "request_id", "u64",
            "tokens", "u32"
        ));
        pipeline.fault("InferenceFault", List.of("UpstreamTimeout", "ParseError"));

        // -- Nodes ------------------------------------------------------------
        pipeline.sink(3080, "/trigger", "webhook");
        pipeline.source("http://localhost:4545/v1/chat/completions",
            "sse", "openai", "choices[0].delta.content", "inference");

        // -- Tri-Lane routes --------------------------------------------------
        pipeline.route("webhook.trigger_out", "inference.trigger_in", "LoanWrite");
        pipeline.route("inference.data_out", "webhook.data_in", "LoanWrite");
        pipeline.route("inference.ctrl_out", "webhook.ctrl_in", "Copy");

        // -- Emit / compile ---------------------------------------------------
        if ("manifest".equals(System.getenv("VIL_COMPILE_MODE"))) {
            System.out.println(pipeline.toYaml());
        } else {
            pipeline.compile();
        }
    }
}
