#!/usr/bin/env tsx
// 509-villog-phase1-integration — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 509-villog-phase1-integration.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("app", 8080);
server.compile();
