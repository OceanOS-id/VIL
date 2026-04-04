#!/usr/bin/env python3
"""005-basic-multiservice-mesh-ndjson — Python SDK equivalent
Compile: vil compile --from python --input 005-basic-multiservice-mesh-ndjson.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

pipeline = VilPipeline("MultiServiceMesh", 3084)
pipeline.sink(port=3084, path="/ingest", name="gateway")
pipeline.source(format="json", name="credit_ingest")
pipeline.route("gateway.trigger_out", "ingest.trigger_in", "LoanWrite")
pipeline.route("ingest.response_data_out", "gateway.response_data_in", "LoanWrite")
pipeline.route("ingest.response_ctrl_out", "gateway.response_ctrl_in", "Copy")
pipeline.compile()
