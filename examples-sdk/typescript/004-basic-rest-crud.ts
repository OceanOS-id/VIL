#!/usr/bin/env tsx
// 004 — REST CRUD (ServiceProcess + State)
// Equivalent to: examples/004-basic-rest-crud (Rust)
// Compile: vil compile --from typescript --input 004-basic-rest-crud.ts --release

import { VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("crud-service", 8080);

// -- Semantic types -----------------------------------------------------------
server.semanticType("TaskState", "state", {
  task_count: "u32",
  last_modified: "u64",
});
server.fault("CrudFault", ["NotFound", "InvalidInput", "Conflict"]);

// -- ServiceProcess: tasks (prefix: /api) -------------------------------------
const tasks = new ServiceProcess("tasks");
tasks.endpoint("GET", "/tasks", "list_tasks");
tasks.endpoint("POST", "/tasks", "create_task");
tasks.endpoint("GET", "/tasks/:id", "get_task");
tasks.endpoint("PUT", "/tasks/:id", "update_task");
tasks.endpoint("DELETE", "/tasks/:id", "delete_task");
server.service(tasks, "/api");

// -- Emit / compile -----------------------------------------------------------
if (process.env.VIL_COMPILE_MODE === "manifest") {
  console.log(server.toYaml());
} else {
  server.compile();
}
