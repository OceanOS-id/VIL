// 005 — Multiservice Mesh (NDJSON + Transform)
// Equivalent to: examples/005-basic-multiservice-mesh-ndjson (Rust)
// Compile: vil compile --from java --input 005/Main.java --release
package dev.vil.examples;

import dev.vil.VilPipeline;
import java.util.Map;
import java.util.List;

public class Main {
    public static void main(String[] args) {
        VilPipeline pipeline = new VilPipeline("multiservice-mesh", 3084, "shm");

        // -- Semantic types ---------------------------------------------------
        pipeline.semanticType("MeshState", "state", Map.of(
            "request_id", "u64",
            "messages_forwarded", "u32",
            "active_pipelines", "u8"
        ));
        pipeline.fault("MeshFault", List.of(
            "ProcessorTimeout", "AnalyticsTimeout", "ShmWriteFailed", "RouteNotFound"
        ));

        // -- Nodes ------------------------------------------------------------
        pipeline.sink(3084, "/ingest", "gateway");
        pipeline.source("http://127.0.0.1:4545/v1/chat/completions",
            "sse", "openai", "choices[0].delta.content", "processor");

        // -- Transform: NDJSON enrichment -------------------------------------
        pipeline.transform("enrich", """
            let mut obj = serde_json::from_slice(line).ok()?;
            obj["processed"] = serde_json::json!(true);
            Some(serde_json::to_vec(&obj).ok()?)
            """);

        // -- Tri-Lane routes --------------------------------------------------
        pipeline.route("gateway.trigger_out", "processor.trigger_in", "LoanWrite");
        pipeline.route("processor.data_out", "enrich.data_in", "LoanWrite");
        pipeline.route("enrich.data_out", "gateway.data_in", "LoanWrite");
        pipeline.route("processor.ctrl_out", "gateway.ctrl_in", "Copy");

        // -- Emit / compile ---------------------------------------------------
        if ("manifest".equals(System.getenv("VIL_COMPILE_MODE"))) {
            System.out.println(pipeline.toYaml());
        } else {
            pipeline.compile();
        }
    }
}
