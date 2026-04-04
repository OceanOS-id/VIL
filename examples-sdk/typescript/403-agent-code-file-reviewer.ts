#!/usr/bin/env tsx
// 403-agent-code-file-reviewer — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 403-agent-code-file-reviewer.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("code-file-reviewer-agent", 3122);
const code_review_agent = new ServiceProcess("code-review-agent");
code_review_agent.endpoint("POST", "/code-review", "code_review_handler");
server.service(code_review_agent);
server.compile();
