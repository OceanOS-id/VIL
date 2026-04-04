#!/usr/bin/env tsx
// 101c-vilapp-multi-pipeline-benchmark — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 101c-vilapp-multi-pipeline-benchmark.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("multi-pipeline-bench", 3090);
const pipeline = new ServiceProcess("pipeline");
server.service(pipeline);
server.compile();
