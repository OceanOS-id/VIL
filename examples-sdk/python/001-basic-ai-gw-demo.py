#!/usr/bin/env python3
"""001 — AI Gateway (SSE Pipeline)
Equivalent to: examples/001-basic-ai-gw-demo (Rust)
Compile: vil compile --from python --input 001-basic-ai-gw-demo.py --release
"""
import os
from vil import VilPipeline

pipeline = VilPipeline("ai-gateway", port=3080)

# -- Semantic types -----------------------------------------------------------
pipeline.semantic_type("InferenceState", "state", fields={
    "request_id": "u64",
    "tokens": "u32",
})
pipeline.fault("InferenceFault", variants=["UpstreamTimeout", "ParseError"])

# -- Nodes --------------------------------------------------------------------
pipeline.sink(port=3080, path="/trigger", name="webhook")
pipeline.source(
    url="http://localhost:4545/v1/chat/completions",
    format="sse",
    dialect="openai",
    json_tap="choices[0].delta.content",
    name="inference",
)

# -- Tri-Lane routes ----------------------------------------------------------
pipeline.route("webhook.trigger_out", "inference.trigger_in", "LoanWrite")
pipeline.route("inference.data_out", "webhook.data_in", "LoanWrite")
pipeline.route("inference.ctrl_out", "webhook.ctrl_in", "Copy")

# -- Emit / compile -----------------------------------------------------------
if os.environ.get("VIL_COMPILE_MODE") == "manifest":
    print(pipeline.to_yaml())
else:
    pipeline.compile()
