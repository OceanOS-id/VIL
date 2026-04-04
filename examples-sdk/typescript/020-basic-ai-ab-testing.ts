#!/usr/bin/env tsx
// 020-basic-ai-ab-testing — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 020-basic-ai-ab-testing.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("ai-ab-testing-gateway", 8080);
const ab = new ServiceProcess("ab");
ab.endpoint("POST", "/infer", "infer");
ab.endpoint("GET", "/metrics", "metrics");
ab.endpoint("POST", "/config", "update_config");
server.service(ab);
const root = new ServiceProcess("root");
root.endpoint("GET", "/", "index");
server.service(root);
server.compile();
