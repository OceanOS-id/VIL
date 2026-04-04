#!/usr/bin/env tsx
// 034-basic-blocking-task — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 034-basic-blocking-task.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("credit-risk-scoring-engine", 8080);
const risk_engine = new ServiceProcess("risk-engine");
risk_engine.endpoint("POST", "/risk/assess", "assess_risk");
risk_engine.endpoint("GET", "/risk/health", "risk_health");
server.service(risk_engine);
server.compile();
