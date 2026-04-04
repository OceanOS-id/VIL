#!/usr/bin/env python3
"""004 — REST CRUD (ServiceProcess + State)
Equivalent to: examples/004-basic-rest-crud (Rust)
Compile: vil compile --from python --input 004-basic-rest-crud.py --release
"""
import os
from vil import VilServer

server = VilServer("crud-service", port=8080)

# -- Semantic types -----------------------------------------------------------
server.semantic_type("TaskState", "state", fields={
    "task_count": "u32",
    "last_modified": "u64",
})
server.fault("CrudFault", variants=["NotFound", "InvalidInput", "Conflict"])

# -- ServiceProcess: tasks (prefix: /api) -------------------------------------
tasks = server.service_process("tasks", prefix="/api")
tasks.endpoint("GET", "/tasks", "list_tasks")
tasks.endpoint("POST", "/tasks", "create_task")
tasks.endpoint("GET", "/tasks/:id", "get_task")
tasks.endpoint("PUT", "/tasks/:id", "update_task")
tasks.endpoint("DELETE", "/tasks/:id", "delete_task")

# -- Emit / compile -----------------------------------------------------------
if os.environ.get("VIL_COMPILE_MODE") == "manifest":
    print(server.to_yaml())
else:
    server.compile()
