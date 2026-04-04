#!/usr/bin/env python3
"""037-basic-vilmodel-derive — Python SDK equivalent
Compile: vil compile --from python --input 037-basic-vilmodel-derive.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("insurance-claim-processing", port=8080)
claims = server.service_process("claims")
claims.endpoint("POST", "/claims/submit", "submit_claim")
claims.endpoint("GET", "/claims/sample", "sample_claim")
server.compile()
