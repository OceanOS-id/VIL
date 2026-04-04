#!/usr/bin/env tsx
// 604-db-elastic-search — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 604-db-elastic-search.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("app", 8080);
server.compile();
