#!/usr/bin/env tsx
// 404-agent-data-csv-analyst — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 404-agent-data-csv-analyst.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("csv-analyst-agent", 3123);
const csv_analyst_agent = new ServiceProcess("csv-analyst-agent");
csv_analyst_agent.endpoint("POST", "/csv-analyze", "csv_analyze_handler");
server.service(csv_analyst_agent);
server.compile();
