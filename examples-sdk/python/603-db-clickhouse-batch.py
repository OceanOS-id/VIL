#!/usr/bin/env python3
"""603-db-clickhouse-batch — Python SDK equivalent
Compile: vil compile --from python --input 603-db-clickhouse-batch.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("app", port=8080)
server.compile()
