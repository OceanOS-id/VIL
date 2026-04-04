#!/usr/bin/env python3
"""607-db-vilorm-multitenant — Python SDK equivalent
Compile: vil compile --from python --input 607-db-vilorm-multitenant.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("vilorm-multitenant", port=8087)
saas = server.service_process("saas")
saas.endpoint("POST", "/tenants", "create_tenant")
saas.endpoint("GET", "/tenants/:id", "get_tenant")
saas.endpoint("PUT", "/tenants/:id", "update_tenant")
saas.endpoint("POST", "/tenants/:id/users", "add_user")
saas.endpoint("GET", "/tenants/:id/users", "list_users")
saas.endpoint("POST", "/tenants/:id/settings", "upsert_setting")
saas.endpoint("GET", "/tenants/:id/settings", "list_settings")
saas.endpoint("GET", "/tenants/:id/stats", "tenant_stats")
server.compile()
