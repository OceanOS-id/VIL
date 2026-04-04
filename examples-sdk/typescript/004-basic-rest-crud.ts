#!/usr/bin/env tsx
// 004-basic-rest-crud — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 004-basic-rest-crud.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("crud-vilorm", 8080);
const tasks = new ServiceProcess("tasks");
tasks.endpoint("GET", "/tasks", "list_tasks");
tasks.endpoint("POST", "/tasks", "create_task");
tasks.endpoint("GET", "/tasks/stats", "task_stats");
tasks.endpoint("GET", "/tasks/:id", "get_task");
tasks.endpoint("PUT", "/tasks/:id", "update_task");
tasks.endpoint("DELETE", "/tasks/:id", "delete_task");
server.service(tasks);
server.compile();
