#!/usr/bin/env tsx
// 024-basic-llm-chat — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 024-basic-llm-chat.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("llm-chat", 8080);
const chat = new ServiceProcess("chat");
chat.endpoint("POST", "/chat", "chat_handler");
server.service(chat);
server.compile();
