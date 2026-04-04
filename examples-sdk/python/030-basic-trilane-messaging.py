#!/usr/bin/env python3
"""030-basic-trilane-messaging — Python SDK equivalent
Compile: vil compile --from python --input 030-basic-trilane-messaging.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("ecommerce-order-pipeline", port=8080)
gateway = server.service_process("gateway")
fulfillment = server.service_process("fulfillment")
fulfillment.endpoint("GET", "/status", "fulfillment_status")
server.compile()
