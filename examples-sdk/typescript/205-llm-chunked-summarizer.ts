#!/usr/bin/env tsx
// 205-llm-chunked-summarizer — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 205-llm-chunked-summarizer.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const p = new VilPipeline("ChunkedSummarizerPipeline", 8080);
p.route("sink.trigger_out", "source_summarize.trigger_in", "LoanWrite");
p.route("source_summarize.response_data_out", "sink.response_data_in", "LoanWrite");
p.route("source_summarize.response_ctrl_out", "sink.response_ctrl_in", "Copy");
p.compile();
