#!/usr/bin/env tsx
// 101b-multi-pipeline-benchmark — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 101b-multi-pipeline-benchmark.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const p = new VilPipeline("MultiPipelineBench", 3090);
p.sink({ port: 3090, path: "/trigger", name: "gateway" });
p.source({ name: "l_l_m_upstream", url: "http://127.0.0.1:4545/v1/chat/completions", jsonTap: "choices[0].delta.content" });
p.route("sink.trigger_out", "source.trigger_in", "LoanWrite");
p.route("source.response_data_out", "sink.response_data_in", "LoanWrite");
p.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy");
p.compile();
