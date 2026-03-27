#!/usr/bin/env tsx
// 001 — AI Gateway (SSE Pipeline)
// Equivalent to: examples/001-basic-ai-gw-demo (Rust)
// Compile: vil compile --from typescript --input 001-basic-ai-gw-demo.ts --release

import { VilPipeline } from "vil-sdk";

const pipeline = new VilPipeline("ai-gateway", 3080);

// -- Semantic types -----------------------------------------------------------
pipeline.semanticType("InferenceState", "state", {
  request_id: "u64",
  tokens: "u32",
});
pipeline.fault("InferenceFault", ["UpstreamTimeout", "ParseError"]);

// -- Nodes --------------------------------------------------------------------
pipeline.sink({ port: 3080, path: "/trigger", name: "webhook" });
pipeline.source({
  url: "http://localhost:4545/v1/chat/completions",
  format: "sse",
  dialect: "openai",
  jsonTap: "choices[0].delta.content",
  name: "inference",
});

// -- Tri-Lane routes ----------------------------------------------------------
pipeline.route("webhook.trigger_out", "inference.trigger_in", "LoanWrite");
pipeline.route("inference.data_out", "webhook.data_in", "LoanWrite");
pipeline.route("inference.ctrl_out", "webhook.ctrl_in", "Copy");

// -- Emit / compile -----------------------------------------------------------
if (process.env.VIL_COMPILE_MODE === "manifest") {
  console.log(pipeline.toYaml());
} else {
  pipeline.compile();
}
