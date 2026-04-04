#!/usr/bin/env tsx
// 202-llm-multi-model-routing — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 202-llm-multi-model-routing.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("MultiModelPipeline_GPT4", 8080);
server.compile();
