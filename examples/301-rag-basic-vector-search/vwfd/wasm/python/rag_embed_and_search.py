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


def main():
    """Entry point: read JSON input, execute search, write JSON output."""
    raw = sys.stdin.read()
    input_data = json.loads(raw) if raw.strip() else {}

    query = input_data.get("query", "")
    top_k = input_data.get("top_k", 3)

    results = search(query, top_k)

    output = {
        "results": results,
        "query": query,
        "total_searched": len(KNOWLEDGE_BASE),
        "embedding_dim": 4
    }
    print(json.dumps(output))


if __name__ == "__main__":
    main()
