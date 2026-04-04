#!/usr/bin/env python3
"""005 — Multiservice Mesh (NDJSON + Transform)
Equivalent to: examples/005-basic-multiservice-mesh-ndjson (Rust)
Compile: vil compile --from python --input 005-basic-multiservice-mesh-ndjson.py --release
"""
import os
from vil import VilPipeline

pipeline = VilPipeline("multiservice-mesh", port=3084)

# -- Semantic types -----------------------------------------------------------
pipeline.semantic_type("MeshState", "state", fields={
    "request_id": "u64",
    "messages_forwarded": "u32",
    "active_pipelines": "u8",
})
pipeline.fault("MeshFault", variants=[
    "ProcessorTimeout", "AnalyticsTimeout", "ShmWriteFailed", "RouteNotFound",
])

# -- Nodes --------------------------------------------------------------------
pipeline.sink(port=3084, path="/ingest", name="gateway")
pipeline.source(
    url="http://127.0.0.1:4545/v1/chat/completions",
    format="sse",
    dialect="openai",
    json_tap="choices[0].delta.content",
    name="processor",
)

# -- Transform: NDJSON enrichment ---------------------------------------------
pipeline.transform("enrich", """
    let mut obj = serde_json::from_slice(line).ok()?;
    obj["processed"] = serde_json::json!(true);
    Some(serde_json::to_vec(&obj).ok()?)
""")

# -- Tri-Lane routes ----------------------------------------------------------
pipeline.route("gateway.trigger_out", "processor.trigger_in", "LoanWrite")
pipeline.route("processor.data_out", "enrich.data_in", "LoanWrite")
pipeline.route("enrich.data_out", "gateway.data_in", "LoanWrite")
pipeline.route("processor.ctrl_out", "gateway.ctrl_in", "Copy")

# -- Emit / compile -----------------------------------------------------------
if os.environ.get("VIL_COMPILE_MODE") == "manifest":
    print(pipeline.to_yaml())
else:
    pipeline.compile()
