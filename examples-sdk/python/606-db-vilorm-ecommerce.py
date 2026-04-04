#!/usr/bin/env python3
"""606-db-vilorm-ecommerce — Python SDK equivalent
Compile: vil compile --from python --input 606-db-vilorm-ecommerce.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("vilorm-ecommerce", port=8086)
shop = server.service_process("shop")
shop.endpoint("POST", "/products", "create_product")
shop.endpoint("GET", "/products", "list_products")
shop.endpoint("GET", "/products/:id", "get_product")
shop.endpoint("POST", "/orders", "create_order")
shop.endpoint("GET", "/orders", "list_orders")
shop.endpoint("GET", "/orders/:id/total", "order_total")
shop.endpoint("DELETE", "/orders/:id", "cancel_order")
server.compile()
