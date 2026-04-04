#!/usr/bin/env tsx
// 008-basic-credit-quality-monitor — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 008-basic-credit-quality-monitor.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const p = new VilPipeline("CreditQualityMonitorPipeline", 3082);
p.sink({ port: 3082, path: "/quality-check", name: "quality_monitor_sink" });
p.source({ name: "quality_credit_source", format: "json" });
p.route("sink.trigger_out", "source.trigger_in", "LoanWrite");
p.route("source.response_data_out", "sink.response_data_in", "LoanWrite");
p.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy");
p.compile();
