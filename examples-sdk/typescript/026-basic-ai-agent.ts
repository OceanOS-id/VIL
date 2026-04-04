#!/usr/bin/env tsx
// 026-basic-ai-agent — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 026-basic-ai-agent.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("ai-agent", 8080);
const agent = new ServiceProcess("agent");
agent.endpoint("POST", "/agent", "agent_handler");
server.service(agent);
server.compile();
