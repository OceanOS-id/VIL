#!/usr/bin/env tsx
// 206-llm-decision-routing — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 206-llm-decision-routing.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("insurance-underwriting-ai", 3116);
const underwriter = new ServiceProcess("underwriter");
server.service(underwriter);
server.compile();
