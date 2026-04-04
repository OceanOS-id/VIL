#!/usr/bin/env tsx
// 101-pipeline-3node-transform-chain — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 101-pipeline-3node-transform-chain.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const p = new VilPipeline("TransformChainPipeline", 3090);
p.sink({ port: 3090, path: "/transform", name: "transform_gateway" });
p.source({ name: "chained_transform_source", url: "http://localhost:18081/api/v1/credits/ndjson?count=100", format: "json" });
p.route("sink.trigger_out", "source.trigger_in", "LoanWrite");
p.route("source.response_data_out", "sink.response_data_in", "LoanWrite");
p.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy");
p.compile();
