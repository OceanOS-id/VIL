#!/usr/bin/env tsx
// 405-agent-react-multi-tool — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 405-agent-react-multi-tool.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("react-multi-tool-agent", 3124);
const react_agent = new ServiceProcess("react-agent");
react_agent.endpoint("POST", "/react", "react_handler");
server.service(react_agent);
server.compile();
