#!/usr/bin/env python3
"""602-db-mongo-crud — Python SDK equivalent
Compile: vil compile --from python --input 602-db-mongo-crud.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("app", port=8080)
server.compile()
