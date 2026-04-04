#!/usr/bin/env tsx
// 027 — VilServer Minimal (No VX)
// Equivalent to: examples/027-basic-vilserver-minimal (Rust)
// Compile: vil compile --from typescript --input 027-basic-vilserver-minimal.ts --release

import { VilServer } from "vil-sdk";

const server = new VilServer("minimal-api", 8080);

// -- Fault type ---------------------------------------------------------------
server.fault("ApiFault", ["InvalidInput", "NotFound"]);

// -- Routes (no ServiceProcess, no VX) ----------------------------------------
server.get("/hello", { handler: "hello" });
server.post("/echo", { handler: "echo" });

// Built-in: GET /health, /ready, /metrics, /info

// -- Emit / compile -----------------------------------------------------------
if (process.env.VIL_COMPILE_MODE === "manifest") {
  console.log(server.toYaml());
} else {
  server.compile();
}
