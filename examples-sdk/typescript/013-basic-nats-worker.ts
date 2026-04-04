#!/usr/bin/env tsx
// 013-basic-nats-worker — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 013-basic-nats-worker.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("nats-worker", 8080);
const nats = new ServiceProcess("nats");
nats.endpoint("GET", "/nats/config", "nats_config");
nats.endpoint("POST", "/nats/publish", "nats_publish");
nats.endpoint("GET", "/nats/jetstream", "jetstream_info");
nats.endpoint("GET", "/nats/kv", "kv_demo");
server.service(nats);
const root = new ServiceProcess("root");
server.service(root);
server.compile();
