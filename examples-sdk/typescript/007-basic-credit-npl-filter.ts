#!/usr/bin/env tsx
// 007-basic-credit-npl-filter — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 007-basic-credit-npl-filter.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const p = new VilPipeline("NplFilterPipeline", 3081);
p.sink({ port: 3081, path: "/filter-npl", name: "npl_filter_sink" });
p.source({ name: "npl_credit_source", format: "json" });
p.route("sink.trigger_out", "source.trigger_in", "LoanWrite");
p.route("source.response_data_out", "sink.response_data_in", "LoanWrite");
p.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy");
p.compile();
