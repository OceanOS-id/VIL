#!/usr/bin/env tsx
// 035-basic-vil-service-module — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 035-basic-vil-service-module.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("app", 8080);
server.compile();
