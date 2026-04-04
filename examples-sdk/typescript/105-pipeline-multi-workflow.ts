#!/usr/bin/env tsx
// 105-pipeline-multi-workflow — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 105-pipeline-multi-workflow.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const p = new VilPipeline("AiGatewayWorkflow", 3097);
p.sink({ port: 3097, path: "/ai", name: "ai_gateway_sink" });
p.source({ name: "ai_sse_source", url: "http://127.0.0.1:4545/v1/chat/completions", format: "sse", jsonTap: "choices[0].delta.content", dialect: "openai" });
p.sink({ port: 3098, path: "/credit", name: "credit_sink" });
p.source({ name: "credit_ndjson_source", url: "http://localhost:18081/api/v1/credits/ndjson?count=100", format: "json" });
p.sink({ port: 3099, path: "/inventory", name: "inventory_sink" });
p.source({ name: "inventory_rest_source", url: "http://localhost:18092/api/v1/products" });
p.route("ai_sink.trigger_out", "ai_source.trigger_in", "LoanWrite");
p.route("ai_source.response_data_out", "ai_sink.response_data_in", "LoanWrite");
p.route("ai_source.response_ctrl_out", "ai_sink.response_ctrl_in", "Copy");
p.route("credit_sink.trigger_out", "credit_source.trigger_in", "LoanWrite");
p.route("credit_source.response_data_out", "credit_sink.response_data_in", "LoanWrite");
p.route("credit_source.response_ctrl_out", "credit_sink.response_ctrl_in", "Copy");
p.route("inventory_sink.trigger_out", "inventory_source.trigger_in", "LoanWrite");
p.route("inventory_source.response_data_out", "inventory_sink.response_data_in", "LoanWrite");
p.route("inventory_source.response_ctrl_out", "inventory_sink.response_ctrl_in", "Copy");
p.compile();
