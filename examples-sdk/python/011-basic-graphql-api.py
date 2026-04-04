#!/usr/bin/env python3
"""011-basic-graphql-api — Python SDK equivalent
Compile: vil compile --from python --input 011-basic-graphql-api.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("graphql-api", port=8080)
graphql = server.service_process("graphql")
graphql.endpoint("GET", "/", "index")
graphql.endpoint("GET", "/schema", "schema_info")
graphql.endpoint("GET", "/entities", "list_entities")
graphql.endpoint("POST", "/query", "graphql_query")
server.compile()
