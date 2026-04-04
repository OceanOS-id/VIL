#!/usr/bin/env tsx
// 603-db-clickhouse-batch — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 603-db-clickhouse-batch.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("app", 8080);
server.compile();
