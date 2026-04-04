#!/usr/bin/env tsx
// 701-mq-rabbitmq-pubsub — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 701-mq-rabbitmq-pubsub.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("app", 8080);
server.compile();
