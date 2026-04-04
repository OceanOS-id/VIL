#!/usr/bin/env tsx
// 102-pipeline-fanout-scatter — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 102-pipeline-fanout-scatter.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const p = new VilPipeline("NplPipeline", 3091);
p.sink({ port: 3091, path: "/npl", name: "npl_sink" });
p.source({ name: "npl_source", url: "http://localhost:18081/api/v1/credits/ndjson?count=100", format: "json" });
p.sink({ port: 3092, path: "/healthy", name: "healthy_sink" });
p.source({ name: "healthy_source", url: "http://localhost:18081/api/v1/credits/ndjson?count=100", format: "json" });
p.route("npl_sink.trigger_out", "npl_source.trigger_in", "LoanWrite");
p.route("npl_source.response_data_out", "npl_sink.response_data_in", "LoanWrite");
p.route("npl_source.response_ctrl_out", "npl_sink.response_ctrl_in", "Copy");
p.route("healthy_sink.trigger_out", "healthy_source.trigger_in", "LoanWrite");
p.route("healthy_source.response_data_out", "healthy_sink.response_data_in", "LoanWrite");
p.route("healthy_source.response_ctrl_out", "healthy_sink.response_ctrl_in", "Copy");
p.compile();
