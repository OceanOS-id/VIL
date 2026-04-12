"""
RAG Full Pipeline — Embed + HNSW Search (Python WASM)

Embeds query using word-hash vectors (64-dim mock),
searches against pre-indexed legal contract sections
using approximate nearest neighbor (simplified HNSW).
Returns top-K with metadata for LLM context building.
"""

import json
import math
import sys


# Pre-indexed legal contract sections (mock HNSW index)
INDEX = [
    {"id": "s1", "section": "§1.1 Definitions", "content": "Effective Date means the date of execution of this Agreement.", "category": "general"},
    {"id": "s2", "section": "§2.3 Payment Terms", "content": "Payment shall be made within 30 days of invoice date via wire transfer.", "category": "payment"},
    {"id": "s3", "section": "§3.1 Confidentiality", "content": "Each party shall maintain confidentiality of proprietary information.", "category": "legal"},
    {"id": "s4", "section": "§4.2 Termination", "content": "Either party may terminate with 90 days written notice.", "category": "termination"},
    {"id": "s5", "section": "§5.1 Indemnification", "content": "Vendor shall indemnify against third-party intellectual property claims.", "category": "legal"},
    {"id": "s6", "section": "§6.3 SLA", "content": "Service availability target: 99.9% uptime measured monthly.", "category": "sla"},
    {"id": "s7", "section": "§7.1 Data Protection", "content": "Personal data processed in accordance with GDPR and local regulations.", "category": "compliance"},
    {"id": "s8", "section": "§8.2 Liability Cap", "content": "Total liability shall not exceed 12 months of fees paid.", "category": "legal"},
]


def embed_64dim(text: str) -> list:
    """Simple 64-dim word hash embedding (mock)."""
    vec = [0.0] * 64
    for i, word in enumerate(text.lower().split()):
        idx = hash(word) % 64
        vec[idx] += 1.0 / (i + 1)
    norm = math.sqrt(sum(x * x for x in vec)) or 1.0
    return [x / norm for x in vec]


def cosine_sim(a: list, b: list) -> float:
    dot = sum(x * y for x, y in zip(a, b))
    na = math.sqrt(sum(x * x for x in a)) or 1.0
    nb = math.sqrt(sum(x * x for x in b)) or 1.0
    return dot / (na * nb)


def search(query: str, top_k: int = 3) -> list:
    q_vec = embed_64dim(query)

    # Pre-compute embeddings for index (in production: stored in HNSW)
    scored = []
    for section in INDEX:
        s_vec = embed_64dim(section["content"])
        score = cosine_sim(q_vec, s_vec)
        scored.append({
            "id": section["id"],
            "section": section["section"],
            "content": section["content"],
            "category": section["category"],
            "score": round(score, 4)
        })

    scored.sort(key=lambda x: x["score"], reverse=True)
    return scored[:top_k]


def main():
    raw = sys.stdin.read()
    input_data = json.loads(raw) if raw.strip() else {}
    query = input_data.get("query", "")
    top_k = input_data.get("top_k", 3)

    results = search(query, top_k)

    print(json.dumps({
        "results": results,
        "query": query,
        "index_size": len(INDEX),
        "embedding_dim": 64,
        "method": "cosine_similarity_hnsw_mock"
    }))


if __name__ == "__main__":
    main()
