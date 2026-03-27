#!/usr/bin/env python3
"""016 — AI RAG Gateway (SSE Pipeline)
Equivalent to: examples/016-basic-ai-rag-gateway (Rust)
Compile: vil compile --from python --input 016-basic-ai-rag-gateway.py --release
"""
import os
from vil import VilPipeline

pipeline = VilPipeline("ai-rag-gateway", port=3084)

# -- Semantic types -----------------------------------------------------------
pipeline.semantic_type("RagState", "state", fields={
    "query_id": "u64",
    "chunks_retrieved": "u32",
    "tokens_generated": "u32",
})
pipeline.fault("RagFault", variants=[
    "VectorDbTimeout", "EmbeddingError", "UpstreamTimeout", "ParseError",
])

# -- Nodes --------------------------------------------------------------------
pipeline.sink(port=3084, path="/rag", name="gateway")
pipeline.source(
    url="http://localhost:4545/v1/chat/completions",
    format="sse",
    dialect="openai",
    json_tap="choices[0].delta.content",
    name="inference",
)

# -- Transform: inject retrieved context before LLM call ----------------------
pipeline.transform("context_inject", """
    let mut req = serde_json::from_slice(line).ok()?;
    let ctx = retrieve_context(&req["query"].as_str()?);
    req["messages"][0]["content"] = serde_json::json!(ctx);
    Some(serde_json::to_vec(&req).ok()?)
""")

# -- Tri-Lane routes ----------------------------------------------------------
pipeline.route("gateway.trigger_out", "context_inject.trigger_in", "LoanWrite")
pipeline.route("context_inject.data_out", "inference.trigger_in", "LoanWrite")
pipeline.route("inference.data_out", "gateway.data_in", "LoanWrite")
pipeline.route("inference.ctrl_out", "gateway.ctrl_in", "Copy")

# -- Emit / compile -----------------------------------------------------------
if os.environ.get("VIL_COMPILE_MODE") == "manifest":
    print(pipeline.to_yaml())
else:
    pipeline.compile()
