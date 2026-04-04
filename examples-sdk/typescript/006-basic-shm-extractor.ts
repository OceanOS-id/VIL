#!/usr/bin/env tsx
// 006-basic-shm-extractor — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 006-basic-shm-extractor.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("shm-extractor-demo", 8080);
const shm_demo = new ServiceProcess("shm-demo");
shm_demo.endpoint("POST", "/ingest", "ingest");
shm_demo.endpoint("POST", "/compute", "compute");
shm_demo.endpoint("GET", "/shm-stats", "shm_stats");
shm_demo.endpoint("GET", "/benchmark", "benchmark");
server.service(shm_demo);
server.compile();
