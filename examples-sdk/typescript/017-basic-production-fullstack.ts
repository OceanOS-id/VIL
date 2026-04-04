#!/usr/bin/env tsx
// 017-basic-production-fullstack — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 017-basic-production-fullstack.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("production-fullstack", 8080);
const fullstack = new ServiceProcess("fullstack");
fullstack.endpoint("GET", "/stack", "stack_info");
fullstack.endpoint("GET", "/config", "full_config");
fullstack.endpoint("GET", "/sprints", "sprints");
fullstack.endpoint("GET", "/middleware", "middleware_info");
server.service(fullstack);
const admin = new ServiceProcess("admin");
admin.endpoint("GET", "/config", "full_config");
server.service(admin);
const root = new ServiceProcess("root");
server.service(root);
server.compile();
