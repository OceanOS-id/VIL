#!/usr/bin/env tsx
// 037-basic-vilmodel-derive — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 037-basic-vilmodel-derive.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("insurance-claim-processing", 8080);
const claims = new ServiceProcess("claims");
claims.endpoint("POST", "/claims/submit", "submit_claim");
claims.endpoint("GET", "/claims/sample", "sample_claim");
server.service(claims);
server.compile();
