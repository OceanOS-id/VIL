#!/usr/bin/env tsx
// 001b-vilapp-ai-gw-benchmark — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 001b-vilapp-ai-gw-benchmark.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("ai-gw-bench", 3081);
const gw = new ServiceProcess("gw");
server.service(gw);
server.compile();
