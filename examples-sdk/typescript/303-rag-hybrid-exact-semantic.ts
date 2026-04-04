#!/usr/bin/env tsx
// 303-rag-hybrid-exact-semantic — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 303-rag-hybrid-exact-semantic.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("rag-hybrid-exact-semantic", 3112);
const rag_hybrid = new ServiceProcess("rag-hybrid");
rag_hybrid.endpoint("POST", "/hybrid", "hybrid_handler");
server.service(rag_hybrid);
server.compile();
