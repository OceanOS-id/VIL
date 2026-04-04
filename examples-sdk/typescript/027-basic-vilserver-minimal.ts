#!/usr/bin/env tsx
// 027-basic-vilserver-minimal — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 027-basic-vilserver-minimal.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("app", 8080);
server.compile();
