#!/usr/bin/env tsx
// 025-basic-rag-service — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 025-basic-rag-service.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("rag-service", 3091);
const rag = new ServiceProcess("rag");
rag.endpoint("POST", "/rag", "rag_handler");
server.service(rag);
server.compile();
