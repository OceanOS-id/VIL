#!/usr/bin/env tsx
// 107-pipeline-process-traced — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 107-pipeline-process-traced.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const p = new VilPipeline("SupplyChainTrackedPipeline", 3107);
p.sink({ port: 3107, path: "/traced", name: "tracking_sink" });
p.source({ name: "supply_chain_source", url: "http://localhost:18081/api/v1/credits/stream", format: "sse" });
p.route("sink.trigger_out", "source.trigger_in", "LoanWrite");
p.route("source.tracking_data_out", "sink.tracking_data_in", "LoanWrite");
p.route("source.delivery_ctrl_out", "sink.delivery_ctrl_in", "Copy");
p.compile();
