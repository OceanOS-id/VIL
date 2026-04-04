#!/usr/bin/env tsx
// 606-db-vilorm-ecommerce — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 606-db-vilorm-ecommerce.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("vilorm-ecommerce", 8086);
const shop = new ServiceProcess("shop");
shop.endpoint("POST", "/products", "create_product");
shop.endpoint("GET", "/products", "list_products");
shop.endpoint("GET", "/products/:id", "get_product");
shop.endpoint("POST", "/orders", "create_order");
shop.endpoint("GET", "/orders", "list_orders");
shop.endpoint("GET", "/orders/:id/total", "order_total");
shop.endpoint("DELETE", "/orders/:id", "cancel_order");
server.service(shop);
server.compile();
