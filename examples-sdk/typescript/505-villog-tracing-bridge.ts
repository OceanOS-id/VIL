#!/usr/bin/env tsx
// 505-villog-tracing-bridge — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 505-villog-tracing-bridge.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("app", 8080);
server.compile();
