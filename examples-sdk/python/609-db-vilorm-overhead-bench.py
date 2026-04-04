#!/usr/bin/env python3
"""609-db-vilorm-overhead-bench — Python SDK equivalent
Compile: vil compile --from python --input 609-db-vilorm-overhead-bench.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("overhead-bench", port=8099)
bench = server.service_process("bench")
bench.endpoint("GET", "/raw/items/:id", "raw_find_by_id")
bench.endpoint("GET", "/raw/items", "raw_list")
bench.endpoint("GET", "/raw/count", "raw_count")
bench.endpoint("GET", "/raw/cols", "raw_select_cols")
bench.endpoint("GET", "/orm/items/:id", "orm_find_by_id")
bench.endpoint("GET", "/orm/items", "orm_list")
bench.endpoint("GET", "/orm/count", "orm_count")
bench.endpoint("GET", "/orm/cols", "orm_select_cols")
server.compile()
