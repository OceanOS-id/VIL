#!/usr/bin/env tsx
// 104-pipeline-diamond-topology — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 104-pipeline-diamond-topology.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const p = new VilPipeline("DiamondSummary", 3095);
p.sink({ port: 3095, path: "/diamond", name: "summary_sink" });
p.source({ name: "summary_source", url: "http://localhost:18081/api/v1/credits/ndjson?count=100", format: "json" });
p.sink({ port: 3096, path: "/diamond-detail", name: "detail_sink" });
p.source({ name: "detail_source", url: "http://localhost:18081/api/v1/credits/ndjson?count=100", format: "json" });
p.route("summary_sink.trigger_out", "summary_source.trigger_in", "LoanWrite");
p.route("summary_source.response_data_out", "summary_sink.response_data_in", "LoanWrite");
p.route("summary_source.response_ctrl_out", "summary_sink.response_ctrl_in", "Copy");
p.route("detail_sink.trigger_out", "detail_source.trigger_in", "LoanWrite");
p.route("detail_source.response_data_out", "detail_sink.response_data_in", "LoanWrite");
p.route("detail_source.response_ctrl_out", "detail_sink.response_ctrl_in", "Copy");
p.compile();
