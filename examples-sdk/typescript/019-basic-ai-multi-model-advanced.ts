#!/usr/bin/env tsx
// 019-basic-ai-multi-model-advanced — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 019-basic-ai-multi-model-advanced.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const p = new VilPipeline("AdvancedMultiModelRouterPipeline", 3086);
p.sink({ port: 3086, path: "/route-advanced", name: "advanced_router_sink" });
p.source({ name: "advanced_router_source", url: "http://127.0.0.1:4545/v1/chat/completions", format: "sse", jsonTap: "choices[0].delta.content", dialect: "openai" });
p.route("sink.trigger_out", "source.trigger_in", "LoanWrite");
p.route("source.response_data_out", "sink.response_data_in", "LoanWrite");
p.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy");
p.compile();
