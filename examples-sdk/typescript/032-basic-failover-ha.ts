#!/usr/bin/env tsx
// 032-basic-failover-ha — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 032-basic-failover-ha.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("payment-gateway-ha", 8080);
const primary = new ServiceProcess("primary");
primary.endpoint("GET", "/health", "primary_health");
primary.endpoint("POST", "/charge", "primary_charge");
server.service(primary);
const backup = new ServiceProcess("backup");
backup.endpoint("GET", "/health", "backup_health");
backup.endpoint("POST", "/charge", "backup_charge");
server.service(backup);
server.compile();
