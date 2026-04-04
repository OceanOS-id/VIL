#!/usr/bin/env tsx
// 014-basic-kafka-stream — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 014-basic-kafka-stream.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("kafka-stream", 8080);
const kafka = new ServiceProcess("kafka");
kafka.endpoint("GET", "/kafka/config", "kafka_config");
kafka.endpoint("POST", "/kafka/produce", "kafka_produce");
kafka.endpoint("GET", "/kafka/consumer", "consumer_info");
kafka.endpoint("GET", "/kafka/bridge", "bridge_status");
server.service(kafka);
const root = new ServiceProcess("root");
server.service(root);
server.compile();
