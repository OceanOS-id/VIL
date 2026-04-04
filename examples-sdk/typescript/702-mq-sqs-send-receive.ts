#!/usr/bin/env tsx
// 702-mq-sqs-send-receive — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 702-mq-sqs-send-receive.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("app", 8080);
server.compile();
