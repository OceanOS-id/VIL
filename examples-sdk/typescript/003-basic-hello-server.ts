#!/usr/bin/env tsx
// 003-basic-hello-server — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 003-basic-hello-server.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("vil-basic-hello-server", 8080);
const gw = new ServiceProcess("gw");
gw.endpoint("POST", "/transform", "transform");
gw.endpoint("POST", "/echo", "echo");
gw.endpoint("GET", "/health", "health");
server.service(gw);
server.compile();
