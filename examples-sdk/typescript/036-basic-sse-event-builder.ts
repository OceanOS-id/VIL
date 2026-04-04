#!/usr/bin/env tsx
// 036-basic-sse-event-builder — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 036-basic-sse-event-builder.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("stock-market-ticker", 8080);
const ticker = new ServiceProcess("ticker");
ticker.endpoint("GET", "/stream", "ticker_stream");
ticker.endpoint("GET", "/info", "ticker_info");
server.service(ticker);
server.compile();
