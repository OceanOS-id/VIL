#!/usr/bin/env tsx
// 401-agent-calculator — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 401-agent-calculator.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("calculator-agent", 3120);
const calc_agent = new ServiceProcess("calc-agent");
calc_agent.endpoint("POST", "/calc", "calc_handler");
server.service(calc_agent);
server.compile();
