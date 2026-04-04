#!/usr/bin/env tsx
// 204-llm-streaming-translator — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 204-llm-streaming-translator.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("llm-streaming-translator", 3103);
const translator = new ServiceProcess("translator");
server.service(translator);
server.compile();
