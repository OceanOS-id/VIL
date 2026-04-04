#!/usr/bin/env python3
"""021-basic-wasm-faas — Python SDK equivalent
Compile: vil compile --from python --input 021-basic-wasm-faas.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("wasm-faas-example", port=8080)
wasm_faas = server.service_process("wasm-faas")
wasm_faas.endpoint("GET", "/", "index")
wasm_faas.endpoint("GET", "/wasm/modules", "list_modules")
wasm_faas.endpoint("POST", "/wasm/pricing", "invoke_pricing")
wasm_faas.endpoint("POST", "/wasm/validation", "invoke_validation")
wasm_faas.endpoint("POST", "/wasm/transform", "invoke_transform")
server.compile()
