#!/usr/bin/env tsx
// 601-storage-s3-basic — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 601-storage-s3-basic.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("app", 8080);
server.compile();
