#!/usr/bin/env python3
"""004-basic-rest-crud — Python SDK equivalent
Compile: vil compile --from python --input 004-basic-rest-crud.py --release

Business: Task CRUD with SQLite + VilORM patterns:
  GET    /tasks       — list tasks (slim projection)
  POST   /tasks       — create task (UUID, validate title)
  GET    /tasks/stats — aggregate statistics (total, done, pending)
  GET    /tasks/:id   — get task by primary key
  PUT    /tasks/:id   — partial update (set_optional)
  DELETE /tasks/:id   — delete by primary key

Handler scripts use sqlite3 for real DB operations matching Rust VilORM.
"""
from vil import VilServer, ServiceProcess, sidecar

server = VilServer("crud-vilorm", port=8080)
tasks = server.service_process("tasks")

tasks.endpoint("GET", "/tasks", "list_tasks",
    impl=sidecar("python3 handlers/list_tasks.py", protocol="shm"))

tasks.endpoint("POST", "/tasks", "create_task",
    impl=sidecar("python3 handlers/create_task.py", protocol="shm"))

tasks.endpoint("GET", "/tasks/stats", "task_stats",
    impl=sidecar("python3 handlers/task_stats.py", protocol="shm"))

tasks.endpoint("GET", "/tasks/:id", "get_task",
    impl=sidecar("python3 handlers/get_task.py", protocol="shm"))

tasks.endpoint("PUT", "/tasks/:id", "update_task",
    impl=sidecar("python3 handlers/update_task.py", protocol="shm"))

tasks.endpoint("DELETE", "/tasks/:id", "delete_task",
    impl=sidecar("python3 handlers/delete_task.py", protocol="shm"))

server.compile()
