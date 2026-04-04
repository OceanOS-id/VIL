#!/usr/bin/env python3
"""106-pipeline-sse-standard-dialect — Python SDK equivalent
Compile: vil compile --from python --input 106-pipeline-sse-standard-dialect.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

pipeline = VilPipeline("IoTSensorPipeline", 3106)
pipeline.sink(port=3106, path="/stream", name="io_t_dashboard_sink")
pipeline.source(url="http://localhost:18081/api/v1/credits/stream", format="sse", name="io_t_sensor_source")
pipeline.route("sink.trigger_out", "source.trigger_in", "LoanWrite")
pipeline.route("source.sensor_data_out", "sink.sensor_data_in", "LoanWrite")
pipeline.route("source.batch_ctrl_out", "sink.batch_ctrl_in", "Copy")
pipeline.compile()
