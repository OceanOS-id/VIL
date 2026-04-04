#!/usr/bin/env tsx
// 028-basic-sse-hub-streaming — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 028-basic-sse-hub-streaming.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("sse-hub-demo", 8080);
const events = new ServiceProcess("events");
events.endpoint("POST", "/publish", "publish");
events.endpoint("GET", "/stream", "stream");
events.endpoint("GET", "/stats", "stats");
server.service(events);
server.compile();
