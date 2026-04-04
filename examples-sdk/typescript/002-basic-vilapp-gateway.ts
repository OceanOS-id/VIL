#!/usr/bin/env tsx
// 002-basic-vilapp-gateway — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 002-basic-vilapp-gateway.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("vil-app-gateway", 3081);
const gw = new ServiceProcess("gw");
gw.endpoint("POST", "/trigger", "trigger_handler");
server.service(gw);
server.compile();
