// 007 — Credit NPL Filter (.transform)
// Equivalent to: examples/007-basic-credit-npl-filter (Rust)
// Compile: vil compile --from java --input 007/Main.java --release
package dev.vil.examples;

import dev.vil.VilPipeline;
import java.util.Map;
import java.util.List;

public class Main {
    public static void main(String[] args) {
        VilPipeline pipeline = new VilPipeline("credit-npl-filter", 3081);

        // -- Semantic types ---------------------------------------------------
        pipeline.semanticType("CreditRecord", "event", Map.of(
            "loan_id", "u64",
            "kolektabilitas", "u8",
            "outstanding", "f64"
        ));
        pipeline.fault("FilterFault", List.of("ParseError", "UpstreamTimeout"));

        // -- Nodes ------------------------------------------------------------
        pipeline.source("http://localhost:18081/api/v1/credits/ndjson?count=1000",
            "ndjson", "credits");
        pipeline.sink(3081, "/filter", "output");

        // -- Transform: keep only NPL (kolektabilitas >= 3) -------------------
        pipeline.transform("npl_filter", """
            let record = serde_json::from_slice(line).ok()?;
            if record["kolektabilitas"].as_u64()? >= 3 { Some(line.to_vec()) } else { None }
            """);

        // -- Tri-Lane routes --------------------------------------------------
        pipeline.route("credits.data_out", "npl_filter.data_in", "LoanWrite");
        pipeline.route("npl_filter.data_out", "output.data_in", "LoanWrite");
        pipeline.route("credits.ctrl_out", "output.ctrl_in", "Copy");

        // -- Emit / compile ---------------------------------------------------
        if ("manifest".equals(System.getenv("VIL_COMPILE_MODE"))) {
            System.out.println(pipeline.toYaml());
        } else {
            pipeline.compile();
        }
    }
}
