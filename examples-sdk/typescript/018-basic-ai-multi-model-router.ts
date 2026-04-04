#!/usr/bin/env tsx
// 018-basic-ai-multi-model-router — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 018-basic-ai-multi-model-router.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("ai-multi-model-router", 3085);
const router = new ServiceProcess("router");
router.endpoint("POST", "/route", "route_handler");
server.service(router);
server.compile();
