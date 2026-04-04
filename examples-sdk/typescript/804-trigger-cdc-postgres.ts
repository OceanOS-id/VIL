#!/usr/bin/env tsx
// 804-trigger-cdc-postgres — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 804-trigger-cdc-postgres.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("app", 8080);
server.compile();
