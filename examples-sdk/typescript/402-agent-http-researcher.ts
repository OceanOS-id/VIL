#!/usr/bin/env tsx
// 402-agent-http-researcher — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 402-agent-http-researcher.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("http-researcher-agent", 3121);
const research_agent = new ServiceProcess("research-agent");
research_agent.endpoint("POST", "/research", "research_handler");
research_agent.endpoint("GET", "/products", "products_handler");
server.service(research_agent);
server.compile();
