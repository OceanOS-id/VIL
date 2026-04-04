#!/usr/bin/env tsx
// 202-llm-multi-model-routing — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 202-llm-multi-model-routing.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const p = new VilPipeline("MultiModelPipeline_GPT4", 8080);
p.route("sink.trigger_out", "source_gpt4.trigger_in", "LoanWrite");
p.route("source_gpt4.response_data_out", "sink.response_data_in", "LoanWrite");
p.route("source_gpt4.response_ctrl_out", "sink.response_ctrl_in", "Copy");
p.compile();
