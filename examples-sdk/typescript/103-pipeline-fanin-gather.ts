#!/usr/bin/env tsx
// 103-pipeline-fanin-gather — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 103-pipeline-fanin-gather.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const p = new VilPipeline("CreditGatherPipeline", 3093);
p.sink({ port: 3093, path: "/gather", name: "credit_gather_sink" });
p.source({ name: "credit_source", url: "http://localhost:18081/api/v1/credits/ndjson?count=100", format: "json" });
p.sink({ port: 3094, path: "/inventory", name: "inventory_gather_sink" });
p.source({ name: "inventory_source", url: "http://localhost:18092/api/v1/products" });
p.route("credit_sink.trigger_out", "credit_source.trigger_in", "LoanWrite");
p.route("credit_source.response_data_out", "credit_sink.response_data_in", "LoanWrite");
p.route("credit_source.response_ctrl_out", "credit_sink.response_ctrl_in", "Copy");
p.route("inventory_sink.trigger_out", "inventory_source.trigger_in", "LoanWrite");
p.route("inventory_source.response_data_out", "inventory_sink.response_data_in", "LoanWrite");
p.route("inventory_source.response_ctrl_out", "inventory_sink.response_ctrl_in", "Copy");
p.compile();
