#!/usr/bin/env tsx
// 205-llm-chunked-summarizer — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 205-llm-chunked-summarizer.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("ChunkedSummarizerPipeline", 8080);
server.compile();
