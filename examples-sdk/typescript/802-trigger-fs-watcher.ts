#!/usr/bin/env tsx
// 802-trigger-fs-watcher — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 802-trigger-fs-watcher.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("app", 8080);
server.compile();
