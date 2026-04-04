#!/usr/bin/env tsx
// 504-villog-benchmark-comparison — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 504-villog-benchmark-comparison.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("app", 8080);
server.compile();
