#!/usr/bin/env tsx
// 021-basic-wasm-faas — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 021-basic-wasm-faas.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("wasm-faas-example", 8080);
const wasm_faas = new ServiceProcess("wasm-faas");
wasm_faas.endpoint("GET", "/", "index");
wasm_faas.endpoint("GET", "/wasm/modules", "list_modules");
wasm_faas.endpoint("POST", "/wasm/pricing", "invoke_pricing");
wasm_faas.endpoint("POST", "/wasm/validation", "invoke_validation");
wasm_faas.endpoint("POST", "/wasm/transform", "invoke_transform");
server.service(wasm_faas);
server.compile();
