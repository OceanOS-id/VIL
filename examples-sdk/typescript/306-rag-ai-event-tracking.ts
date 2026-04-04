#!/usr/bin/env tsx
// 306-rag-ai-event-tracking — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 306-rag-ai-event-tracking.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("customer-support-rag", 3116);
const support = new ServiceProcess("support");
support.endpoint("POST", "/support/ask", "answer_question");
server.service(support);
server.compile();
