#!/usr/bin/env tsx
// 502-villog-file-rolling — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 502-villog-file-rolling.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("app", 8080);
server.compile();
