#!/usr/bin/env tsx
// 801-trigger-cron-basic — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 801-trigger-cron-basic.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("app", 8080);
server.compile();
