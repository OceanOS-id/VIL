#!/usr/bin/env tsx
// 602-db-mongo-crud — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 602-db-mongo-crud.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("app", 8080);
server.compile();
