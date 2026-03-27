#!/usr/bin/env tsx
// 005 — Multiservice Mesh (NDJSON + Transform)
// Equivalent to: examples/005-basic-multiservice-mesh-ndjson (Rust)
// Compile: vil compile --from typescript --input 005-basic-multiservice-mesh-ndjson.ts --release

import { VilPipeline } from "vil-sdk";

const pipeline = new VilPipeline("multiservice-mesh", 3084, { token: "shm" });

// -- Semantic types -----------------------------------------------------------
pipeline.semanticType("MeshState", "state", {
  request_id: "u64",
  messages_forwarded: "u32",
  active_pipelines: "u8",
});
pipeline.fault("MeshFault", [
  "ProcessorTimeout", "AnalyticsTimeout", "ShmWriteFailed", "RouteNotFound",
]);

// -- Nodes --------------------------------------------------------------------
pipeline.sink({ port: 3084, path: "/ingest", name: "gateway" });
pipeline.source({
  url: "http://127.0.0.1:4545/v1/chat/completions",
  format: "sse",
  dialect: "openai",
  jsonTap: "choices[0].delta.content",
  name: "processor",
});

// -- Transform: NDJSON enrichment ---------------------------------------------
pipeline.transform("enrich", `
    let mut obj = serde_json::from_slice(line).ok()?;
    obj["processed"] = serde_json::json!(true);
    Some(serde_json::to_vec(&obj).ok()?)
`);

// -- Tri-Lane routes ----------------------------------------------------------
pipeline.route("gateway.trigger_out", "processor.trigger_in", "LoanWrite");
pipeline.route("processor.data_out", "enrich.data_in", "LoanWrite");
pipeline.route("enrich.data_out", "gateway.data_in", "LoanWrite");
pipeline.route("processor.ctrl_out", "gateway.ctrl_in", "Copy");

// -- Emit / compile -----------------------------------------------------------
if (process.env.VIL_COMPILE_MODE === "manifest") {
  console.log(pipeline.toYaml());
} else {
  pipeline.compile();
}
