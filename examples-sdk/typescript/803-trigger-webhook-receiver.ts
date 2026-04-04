#!/usr/bin/env tsx
// 803-trigger-webhook-receiver — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 803-trigger-webhook-receiver.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("app", 8080);
server.compile();
