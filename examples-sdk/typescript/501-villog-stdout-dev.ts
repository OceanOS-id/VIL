#!/usr/bin/env tsx
// 501-villog-stdout-dev — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 501-villog-stdout-dev.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("app", 8080);
server.compile();
