#!/usr/bin/env tsx
// 029-basic-vil-handler-endpoint — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 029-basic-vil-handler-endpoint.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("macro-demo", 8080);
const demo = new ServiceProcess("demo");
demo.endpoint("GET", "/plain", "plain_handler");
demo.endpoint("GET", "/handled", "handled_handler");
demo.endpoint("POST", "/endpoint", "endpoint_handler");
server.service(demo);
server.compile();
