#!/usr/bin/env tsx
// 031-basic-mesh-routing — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 031-basic-mesh-routing.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("banking-transaction-mesh", 8080);
const teller = new ServiceProcess("teller");
teller.endpoint("GET", "/ping", "teller_ping");
teller.endpoint("POST", "/submit", "teller_submit");
server.service(teller);
const fraud_check = new ServiceProcess("fraud_check");
fraud_check.endpoint("POST", "/analyze", "fraud_process");
server.service(fraud_check);
const core_banking = new ServiceProcess("core_banking");
core_banking.endpoint("POST", "/post", "core_banking_post");
server.service(core_banking);
const notification = new ServiceProcess("notification");
notification.endpoint("GET", "/send", "notification_send");
server.service(notification);
server.compile();
