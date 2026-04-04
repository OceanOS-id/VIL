#!/usr/bin/env tsx
// 023-basic-hybrid-wasm-sidecar — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 023-basic-hybrid-wasm-sidecar.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("hybrid-pipeline", 8080);
const pipeline = new ServiceProcess("pipeline");
pipeline.endpoint("GET", "/", "index");
pipeline.endpoint("POST", "/validate", "validate_order");
pipeline.endpoint("POST", "/price", "calculate_price");
pipeline.endpoint("POST", "/fraud", "fraud_check");
pipeline.endpoint("POST", "/order", "process_order");
server.service(pipeline);
server.compile();
