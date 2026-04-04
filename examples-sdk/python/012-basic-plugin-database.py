#!/usr/bin/env python3
"""012-basic-plugin-database — Python SDK equivalent
Compile: vil compile --from python --input 012-basic-plugin-database.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("plugin-database", port=8080)
plugin_db = server.service_process("plugin-db")
plugin_db.endpoint("GET", "/", "index")
plugin_db.endpoint("GET", "/plugins", "list_plugins")
plugin_db.endpoint("GET", "/config", "show_config")
plugin_db.endpoint("GET", "/products", "list_products")
plugin_db.endpoint("POST", "/tasks", "create_task")
plugin_db.endpoint("GET", "/tasks", "list_tasks")
plugin_db.endpoint("GET", "/pool-stats", "pool_stats")
plugin_db.endpoint("GET", "/redis-ping", "redis_ping")
server.compile()
