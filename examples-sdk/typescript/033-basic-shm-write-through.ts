#!/usr/bin/env tsx
// 033-basic-shm-write-through — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 033-basic-shm-write-through.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("realtime-analytics-dashboard", 8080);
const catalog = new ServiceProcess("catalog");
catalog.endpoint("POST", "/catalog/search", "catalog_search");
catalog.endpoint("GET", "/catalog/health", "catalog_health");
server.service(catalog);
server.compile();
