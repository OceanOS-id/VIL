#!/usr/bin/env python3
"""003-basic-hello-server — Python SDK equivalent
Compile: vil compile --from python --input 003-basic-hello-server.py --release

Business: Simple API gateway with 3 endpoints:
  POST /transform — uppercase input data, double numeric value
  POST /echo     — echo back received body with byte count
  GET  /health   — service health check with SHM status

Handler scripts in handlers/ implement the actual business logic.
"""
from vil import VilServer, ServiceProcess, sidecar

server = VilServer("vil-basic-hello-server", port=8080)
gw = server.service_process("gw")

# POST /transform — uppercase data, double value, add timestamp
gw.endpoint("POST", "/transform", "transform",
    impl=sidecar("python3 handlers/transform.py", protocol="shm"))

# POST /echo — echo back body with byte count + zero_copy flag
gw.endpoint("POST", "/echo", "echo",
    impl=sidecar("python3 handlers/echo.py", protocol="shm"))

# GET /health — service health check with SHM status
gw.endpoint("GET", "/health", "health",
    impl=sidecar("python3 handlers/health.py", protocol="shm"))

server.compile()
