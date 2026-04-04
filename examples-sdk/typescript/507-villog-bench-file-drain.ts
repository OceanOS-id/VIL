#!/usr/bin/env tsx
// 507-villog-bench-file-drain — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 507-villog-bench-file-drain.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("app", 8080);
server.compile();
