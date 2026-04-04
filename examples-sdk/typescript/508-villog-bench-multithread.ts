#!/usr/bin/env tsx
// 508-villog-bench-multithread — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 508-villog-bench-multithread.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("app", 8080);
server.compile();
