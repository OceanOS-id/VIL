#!/usr/bin/env tsx
// 010-basic-websocket-chat — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 010-basic-websocket-chat.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("websocket-chat", 8080);
const chat = new ServiceProcess("chat");
chat.endpoint("GET", "/", "index");
chat.endpoint("GET", "/ws", "ws_handler");
chat.endpoint("GET", "/stats", "stats");
server.service(chat);
server.compile();
