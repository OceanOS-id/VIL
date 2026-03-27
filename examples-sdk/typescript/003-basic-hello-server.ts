#!/usr/bin/env tsx
// 003 — Hello Server (VX_APP)
// Equivalent to: examples/003-basic-hello-server (Rust)
// Compile: vil compile --from typescript --input 003-basic-hello-server.ts --release

import { VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("hello-server", 8080);

// -- ServiceProcess: hello (prefix: /api/hello) -------------------------------
const hello = new ServiceProcess("hello");
hello.endpoint("GET", "/", "hello");
hello.endpoint("GET", "/greet/:name", "greet");
hello.endpoint("POST", "/echo", "echo");
hello.endpoint("GET", "/shm-info", "shm_info");
server.service(hello, "/api/hello");

// Built-in: GET /health, /ready, /metrics, /info

// -- Emit / compile -----------------------------------------------------------
if (process.env.VIL_COMPILE_MODE === "manifest") {
  console.log(server.toYaml());
} else {
  server.compile();
}
