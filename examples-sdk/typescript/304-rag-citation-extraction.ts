#!/usr/bin/env tsx
// 304-rag-citation-extraction — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 304-rag-citation-extraction.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("rag-citation-extraction", 3113);
const rag_citation = new ServiceProcess("rag-citation");
rag_citation.endpoint("POST", "/cited-rag", "cited_rag_handler");
server.service(rag_citation);
server.compile();
