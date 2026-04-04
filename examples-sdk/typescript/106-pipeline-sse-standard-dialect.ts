#!/usr/bin/env tsx
// 106-pipeline-sse-standard-dialect — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 106-pipeline-sse-standard-dialect.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const p = new VilPipeline("IoTSensorPipeline", 3106);
p.sink({ port: 3106, path: "/stream", name: "io_t_dashboard_sink" });
p.source({ name: "io_t_sensor_source", url: "http://localhost:18081/api/v1/credits/stream", format: "sse" });
p.route("sink.trigger_out", "source.trigger_in", "LoanWrite");
p.route("source.sensor_data_out", "sink.sensor_data_in", "LoanWrite");
p.route("source.batch_ctrl_out", "sink.batch_ctrl_in", "Copy");
p.compile();
