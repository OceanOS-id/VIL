#!/usr/bin/env tsx
// 506-villog-structured-events — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 506-villog-structured-events.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("app", 8080);
server.compile();
