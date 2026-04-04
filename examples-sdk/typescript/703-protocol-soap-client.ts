#!/usr/bin/env tsx
// 703-protocol-soap-client — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 703-protocol-soap-client.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("app", 8080);
server.compile();
