#!/usr/bin/env tsx
// 030-basic-trilane-messaging — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 030-basic-trilane-messaging.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("ecommerce-order-pipeline", 8080);
const gateway = new ServiceProcess("gateway");
server.service(gateway);
const fulfillment = new ServiceProcess("fulfillment");
fulfillment.endpoint("GET", "/status", "fulfillment_status");
server.service(fulfillment);
server.compile();
