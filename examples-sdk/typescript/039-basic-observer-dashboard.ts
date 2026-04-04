#!/usr/bin/env tsx
// 039-basic-observer-dashboard — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 039-basic-observer-dashboard.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("observer-demo", 8080);
const demo = new ServiceProcess("demo");
demo.endpoint("GET", "/hello", "hello");
demo.endpoint("POST", "/echo", "echo");
server.service(demo);
server.compile();
