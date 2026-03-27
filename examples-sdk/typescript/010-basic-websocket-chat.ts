#!/usr/bin/env tsx
// 010 — WebSocket Chat
// Equivalent to: examples/010-basic-websocket-chat (Rust)
// Compile: vil compile --from typescript --input 010-basic-websocket-chat.ts --release

import { VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("websocket-chat", 8080);

// -- WebSocket events ---------------------------------------------------------
server.wsEvent("chat_message", {
  topic: "chat.message",
  fields: { from: "String", message: "String", timestamp: "String" },
});
server.wsEvent("user_joined", {
  topic: "chat.presence",
  fields: { username: "String" },
});
server.wsEvent("user_left", {
  topic: "chat.presence",
  fields: { username: "String" },
});

// -- ServiceProcess: chat (prefix: /api/chat) ---------------------------------
const chat = new ServiceProcess("chat");
chat.endpoint("GET", "/", "index");
chat.endpoint("GET", "/ws", "ws_handler", { protocol: "websocket" });
chat.endpoint("GET", "/stats", "stats");
server.service(chat, "/api/chat");

// -- Emit / compile -----------------------------------------------------------
if (process.env.VIL_COMPILE_MODE === "manifest") {
  console.log(server.toYaml());
} else {
  server.compile();
}
