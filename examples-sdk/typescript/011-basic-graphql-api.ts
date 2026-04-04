#!/usr/bin/env tsx
// 011-basic-graphql-api — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 011-basic-graphql-api.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("graphql-api", 8080);
const graphql = new ServiceProcess("graphql");
graphql.endpoint("GET", "/", "index");
graphql.endpoint("GET", "/schema", "schema_info");
graphql.endpoint("GET", "/entities", "list_entities");
graphql.endpoint("POST", "/query", "graphql_query");
server.service(graphql);
server.compile();
