#!/usr/bin/env python3
"""027-basic-vilserver-minimal — Python SDK equivalent
Compile: vil compile --from python --input 027-basic-vilserver-minimal.py --release

Business: Minimal VIL server — hello world + echo endpoint.
  GET  /hello → static greeting string
  POST /echo  → echo back received JSON with byte count
"""
import os
from vil import VilServer, stub

server = VilServer("app", port=8080)

# GET /hello — returns static greeting
server.get("/hello", handler="hello",
    impl=stub(response='{"message": "Hello from VilServer (no VX)!"}'))

# POST /echo — echo back received body with size info
server.post("/echo", handler="echo",
    impl=stub(response='{"received": 0, "echo": null}'))

server.compile()
