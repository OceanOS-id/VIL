#!/usr/bin/env tsx
// 203-llm-code-review-with-tools — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 203-llm-code-review-with-tools.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("llm-code-review-tools", 3102);
const code_review = new ServiceProcess("code-review");
code_review.endpoint("POST", "/code/review", "code_review_handler");
server.service(code_review);
server.compile();
