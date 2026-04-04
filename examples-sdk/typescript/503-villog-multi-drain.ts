#!/usr/bin/env tsx
// 503-villog-multi-drain — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 503-villog-multi-drain.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("app", 8080);
server.compile();
