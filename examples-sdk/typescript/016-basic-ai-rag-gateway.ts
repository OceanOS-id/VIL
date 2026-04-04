#!/usr/bin/env tsx
// 016-basic-ai-rag-gateway — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 016-basic-ai-rag-gateway.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const p = new VilPipeline("RagPipeline", 3084);
p.sink({ port: 3084, path: "/rag", name: "rag_webhook" });
p.source({ name: "rag_sse_inference", url: "http://127.0.0.1:4545/v1/chat/completions", format: "sse", jsonTap: "choices[0].delta.content", dialect: "openai" });
p.route("sink.trigger_out", "source.trigger_in", "LoanWrite");
p.route("source.response_data_out", "sink.response_data_in", "LoanWrite");
p.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy");
p.compile();
