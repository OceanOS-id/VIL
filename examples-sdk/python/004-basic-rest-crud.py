#!/usr/bin/env python3
"""004-basic-rest-crud — Python SDK equivalent
Compile: vil compile --from python --input 004-basic-rest-crud.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("crud-vilorm", port=8080)
tasks = server.service_process("tasks")
tasks.endpoint("GET", "/tasks", "list_tasks")
tasks.endpoint("POST", "/tasks", "create_task")
tasks.endpoint("GET", "/tasks/stats", "task_stats")
tasks.endpoint("GET", "/tasks/:id", "get_task")
tasks.endpoint("PUT", "/tasks/:id", "update_task")
tasks.endpoint("DELETE", "/tasks/:id", "delete_task")
server.compile()
