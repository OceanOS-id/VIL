#!/usr/bin/env tsx
// 301-rag-basic-vector-search — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 301-rag-basic-vector-search.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("rag-basic-vector-search", 3110);
const rag_basic = new ServiceProcess("rag-basic");
rag_basic.endpoint("POST", "/rag", "rag_handler");
server.service(rag_basic);
server.compile();
