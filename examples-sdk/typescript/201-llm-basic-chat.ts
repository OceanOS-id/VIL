#!/usr/bin/env tsx
// 201-llm-basic-chat — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 201-llm-basic-chat.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("llm-basic-chat", 3100);
const chat = new ServiceProcess("chat");
chat.endpoint("POST", "/chat", "chat_handler");
server.service(chat);
server.compile();
