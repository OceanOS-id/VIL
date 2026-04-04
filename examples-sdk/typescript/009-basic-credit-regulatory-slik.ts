#!/usr/bin/env tsx
// 009-basic-credit-regulatory-slik — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 009-basic-credit-regulatory-slik.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const p = new VilPipeline("RegulatoryStreamPipeline", 3083);
p.sink({ port: 3083, path: "/regulatory-stream", name: "regulatory_sink" });
p.source({ name: "regulatory_source", url: "http://localhost:18081/api/v1/credits/ndjson?count=1000", format: "json" });
p.route("sink.trigger_out", "source.trigger_in", "LoanWrite");
p.route("source.response_data_out", "sink.response_data_in", "LoanWrite");
p.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy");
p.compile();
