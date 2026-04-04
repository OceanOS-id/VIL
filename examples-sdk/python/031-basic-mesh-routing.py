#!/usr/bin/env python3
"""031-basic-mesh-routing — Python SDK equivalent
Compile: vil compile --from python --input 031-basic-mesh-routing.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("banking-transaction-mesh", port=8080)
teller = server.service_process("teller")
teller.endpoint("GET", "/ping", "teller_ping")
teller.endpoint("POST", "/submit", "teller_submit")
fraud_check = server.service_process("fraud_check")
fraud_check.endpoint("POST", "/analyze", "fraud_process")
core_banking = server.service_process("core_banking")
core_banking.endpoint("POST", "/post", "core_banking_post")
notification = server.service_process("notification")
notification.endpoint("GET", "/send", "notification_send")
server.compile()
