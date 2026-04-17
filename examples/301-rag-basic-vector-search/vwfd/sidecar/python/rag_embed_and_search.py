"""
RAG Basic Vector Search — Python WASM Module

Embeds query using simple word-hash vector (mock embedding),
computes cosine similarity against a static knowledge base,
returns top-K results with scores.

Compiled to WASM via Pyodide / RustPython-wasm.
Input/Output: JSON via stdin/stdout.
"""

import json
import math
import sys


# Static knowledge base (in production: loaded from vector DB)
KNOWLEDGE_BASE = [
    {
        "id": "doc_1",
        "title": "API Authentication Guide",
        "content": "Use Bearer tokens for API authentication. Configure OAuth2 scopes for fine-grained access control.",
        "embedding": [0.8, 0.2, 0.1, 0.3]
    },
    {
        "id": "doc_2",
        "title": "Rate Limiting Configuration",
        "content": "Configure rate limits per tenant. Default: 100 req/s burst, 50 req/s sustained.",
        "embedding": [0.1, 0.9, 0.3, 0.2]
    },
    {
        "id": "doc_3",
        "title": "Database Connection Pooling",
        "content": "Set max_connections=20, idle_timeout=300s. Use connection pooling for PostgreSQL and MySQL.",
        "embedding": [0.3, 0.1, 0.8, 0.4]
    },
    {
        "id": "doc_4",
        "title": "Error Handling Best Practices",
        "content": "Return structured JSON errors with code, message, and trace_id. Use domain-specific error enums.",
        "embedding": [0.2, 0.4, 0.2, 0.9]
    },
]


def embed_query(query: str) -> list:
    """Simple word-hash embedding (mock). Production: use sentence-transformers."""
    words = query.lower().split()
    vec = [0.0] * 4
    for word in words:
        h = hash(word) % 1000 / 1000.0
        vec[hash(word) % 4] += h
    # L2 normalize
    norm = math.sqrt(sum(x * x for x in vec)) or 1.0
    return [x / norm for x in vec]


def cosine_similarity(a: list, b: list) -> float:
    """Cosine similarity between two vectors."""
    dot = sum(x * y for x, y in zip(a, b))
    norm_a = math.sqrt(sum(x * x for x in a)) or 1.0
    norm_b = math.sqrt(sum(x * x for x in b)) or 1.0
    return dot / (norm_a * norm_b)


def search(query: str, top_k: int = 3) -> list:
    """Embed query → cosine search → return top-K."""
    query_vec = embed_query(query)
    scored = []
    for doc in KNOWLEDGE_BASE:
        score = cosine_similarity(query_vec, doc["embedding"])
        scored.append({
            "doc_id": doc["id"],
            "title": doc["title"],
            "content": doc["content"],
            "score": round(score, 4)
        })
    scored.sort(key=lambda x: x["score"], reverse=True)
    return scored[:top_k]


def process_one(raw):
    """Process one request, execute search, write JSON output."""
    input_data = raw if isinstance(raw, dict) else (json.loads(raw) if str(raw).strip() else {})

    query = input_data.get("query", "")
    top_k = input_data.get("top_k", 3)

    results = search(query, top_k)

    output = {
        "results": results,
        "query": query,
        "total_searched": len(KNOWLEDGE_BASE),
        "embedding_dim": 4
    }
    return output


# ── VIL Sidecar Dual-Mode: UDS+SHM primary, stdin/stdout fallback ──
try:
    import os
    sys.path.insert(0, os.path.join(os.path.dirname(__file__), '../../../../crates/vil_sidecar/sdk'))
    from vil_sidecar_sdk import SidecarApp
    _VIL_SDK = True
except ImportError:
    _VIL_SDK = False


# ── VIL Sidecar: 034 pattern (UDS+SHM primary, stdin/stdout fallback) ──
import os
try:
    sys.path.insert(0, os.path.join(os.path.dirname(__file__), '../../../../crates/vil_sidecar/sdk'))
    from vil_sidecar_sdk import SidecarApp
    VIL_SDK = True
except ImportError:
    VIL_SDK = False

if VIL_SDK and os.environ.get("VIL_SIDECAR_SOCKET"):
    app = SidecarApp("rag_embed_and_search")
    app.handler("execute")(process_one)
    app.run()
else:
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            data = json.loads(line)
            result = process_one(data)
            print(json.dumps(result), flush=True)
        except Exception as e:
            print(json.dumps({"error": str(e)}), flush=True)
