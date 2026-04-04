#!/usr/bin/env tsx
// 038-basic-vil-app-dsl — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 038-basic-vil-app-dsl.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("app", 8080);
server.compile();
