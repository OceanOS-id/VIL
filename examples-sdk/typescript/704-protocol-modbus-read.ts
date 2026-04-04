#!/usr/bin/env tsx
// 704-protocol-modbus-read — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 704-protocol-modbus-read.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("app", 8080);
server.compile();
