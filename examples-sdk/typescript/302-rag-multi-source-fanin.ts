#!/usr/bin/env tsx
// 302-rag-multi-source-fanin — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 302-rag-multi-source-fanin.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const p = new VilPipeline("rag-multi-source-fanin", 3111);
p.compile();
