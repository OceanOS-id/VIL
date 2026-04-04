#!/usr/bin/env tsx
// 022-basic-sidecar-python — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 022-basic-sidecar-python.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("sidecar-python-example", 8080);
const fraud = new ServiceProcess("fraud");
fraud.endpoint("GET", "/status", "fraud_status");
fraud.endpoint("POST", "/check", "fraud_check");
server.service(fraud);
const root = new ServiceProcess("root");
root.endpoint("GET", "/", "index");
server.service(root);
server.compile();
