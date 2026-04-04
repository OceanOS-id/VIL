#!/usr/bin/env tsx
// 305-rag-guardrail-pipeline — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 305-rag-guardrail-pipeline.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("rag-guardrail-pipeline", 3114);
const rag_guardrail = new ServiceProcess("rag-guardrail");
rag_guardrail.endpoint("POST", "/safe-rag", "safe_rag_handler");
server.service(rag_guardrail);
server.compile();
