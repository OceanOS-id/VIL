#!/usr/bin/env tsx
// 406-agent-vil-handler-shm — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 406-agent-vil-handler-shm.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("fraud-detection-agent", 3126);
const fraud_agent = new ServiceProcess("fraud-agent");
fraud_agent.endpoint("POST", "/detect", "detect_fraud");
fraud_agent.endpoint("GET", "/health", "health");
server.service(fraud_agent);
server.compile();
