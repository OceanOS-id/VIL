#!/usr/bin/env python3
"""404-agent-data-csv-analyst — Python SDK equivalent
Compile: vil compile --from python --input 404-agent-data-csv-analyst.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("csv-analyst-agent", port=3123)
csv_analyst_agent = server.service_process("csv-analyst-agent")
csv_analyst_agent.endpoint("POST", "/csv-analyze", "csv_analyze_handler")
server.compile()
