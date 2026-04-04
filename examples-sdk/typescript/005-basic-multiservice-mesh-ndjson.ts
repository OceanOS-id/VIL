#!/usr/bin/env tsx
// 005-basic-multiservice-mesh-ndjson — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 005-basic-multiservice-mesh-ndjson.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const p = new VilPipeline("MultiServiceMesh", 3084);
p.sink({ port: 3084, path: "/ingest", name: "gateway" });
p.source({ name: "credit_ingest", format: "json" });
p.route("gateway.trigger_out", "ingest.trigger_in", "LoanWrite");
p.route("ingest.response_data_out", "gateway.response_data_in", "LoanWrite");
p.route("ingest.response_ctrl_out", "gateway.response_ctrl_in", "Copy");
p.compile();
