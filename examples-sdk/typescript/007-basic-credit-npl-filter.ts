#!/usr/bin/env tsx
// 007 — Credit NPL Filter (.transform)
// Equivalent to: examples/007-basic-credit-npl-filter (Rust)
// Compile: vil compile --from typescript --input 007-basic-credit-npl-filter.ts --release

import { VilPipeline } from "vil-sdk";

const pipeline = new VilPipeline("credit-npl-filter", 3081);

// -- Semantic types -----------------------------------------------------------
pipeline.semanticType("CreditRecord", "event", {
  loan_id: "u64",
  kolektabilitas: "u8",
  outstanding: "f64",
});
pipeline.fault("FilterFault", ["ParseError", "UpstreamTimeout"]);

// -- Nodes --------------------------------------------------------------------
pipeline.source({
  url: "http://localhost:18081/api/v1/credits/ndjson?count=1000",
  format: "ndjson",
  name: "credits",
});
pipeline.sink({ port: 3081, path: "/filter", name: "output" });

// -- Transform: keep only NPL (kolektabilitas >= 3) ---------------------------
pipeline.transform("npl_filter", `
    let record = serde_json::from_slice(line).ok()?;
    if record["kolektabilitas"].as_u64()? >= 3 { Some(line.to_vec()) } else { None }
`);

// -- Tri-Lane routes ----------------------------------------------------------
pipeline.route("credits.data_out", "npl_filter.data_in", "LoanWrite");
pipeline.route("npl_filter.data_out", "output.data_in", "LoanWrite");
pipeline.route("credits.ctrl_out", "output.ctrl_in", "Copy");

// -- Emit / compile -----------------------------------------------------------
if (process.env.VIL_COMPILE_MODE === "manifest") {
  console.log(pipeline.toYaml());
} else {
  pipeline.compile();
}
