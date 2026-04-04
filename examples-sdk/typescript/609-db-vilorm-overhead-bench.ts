#!/usr/bin/env tsx
// 609-db-vilorm-overhead-bench — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 609-db-vilorm-overhead-bench.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("overhead-bench", 8099);
const bench = new ServiceProcess("bench");
bench.endpoint("GET", "/raw/items/:id", "raw_find_by_id");
bench.endpoint("GET", "/raw/items", "raw_list");
bench.endpoint("GET", "/raw/count", "raw_count");
bench.endpoint("GET", "/raw/cols", "raw_select_cols");
bench.endpoint("GET", "/orm/items/:id", "orm_find_by_id");
bench.endpoint("GET", "/orm/items", "orm_list");
bench.endpoint("GET", "/orm/count", "orm_count");
bench.endpoint("GET", "/orm/cols", "orm_select_cols");
server.service(bench);
server.compile();
