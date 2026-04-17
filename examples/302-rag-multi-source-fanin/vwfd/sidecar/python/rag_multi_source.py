"""
RAG Multi-Source Fan-In — Python WASM Module

Searches 2 knowledge bases independently (tech_docs + FAQ),
cross-ranks results using Reciprocal Rank Fusion (RRF),
returns merged top-K.
"""

import json
import sys

TECH_DOCS = [
    {"id": "td_1", "title": "API Gateway Architecture", "keywords": "gateway routing load balancer proxy"},
    {"id": "td_2", "title": "Microservice Communication", "keywords": "grpc rest messaging queue async"},
    {"id": "td_3", "title": "Database Sharding", "keywords": "partition shard distributed database scaling"},
    {"id": "td_4", "title": "Caching Strategy", "keywords": "redis cache ttl invalidation performance"},
]

FAQ = [
    {"id": "faq_1", "question": "How to deploy?", "answer": "Use Docker + K8s", "keywords": "deploy docker kubernetes container"},
    {"id": "faq_2", "question": "How to monitor?", "answer": "Use Prometheus + Grafana", "keywords": "monitor metrics prometheus grafana alert"},
    {"id": "faq_3", "question": "How to scale?", "answer": "Horizontal scaling with load balancer", "keywords": "scale horizontal autoscale load balance"},
]


def keyword_score(query: str, keywords: str) -> float:
    q_words = set(query.lower().split())
    k_words = set(keywords.lower().split())
    overlap = q_words & k_words
    return len(overlap) / max(len(q_words), 1)


def search_source(query: str, source: list, source_name: str) -> list:
    results = []
    for doc in source:
        kw = doc.get("keywords", "")
        score = keyword_score(query, kw)
        if score > 0:
            results.append({"id": doc["id"], "title": doc.get("title", doc.get("question", "")), "score": round(score, 3), "source": source_name})
    results.sort(key=lambda x: x["score"], reverse=True)
    return results


def reciprocal_rank_fusion(results_a: list, results_b: list, k: int = 60) -> list:
    """RRF: score = sum(1 / (k + rank)) across all lists."""
    scores = {}
    for rank, doc in enumerate(results_a):
        doc_id = doc["id"]
        scores[doc_id] = scores.get(doc_id, 0) + 1.0 / (k + rank + 1)
        scores[doc_id + "_data"] = doc
    for rank, doc in enumerate(results_b):
        doc_id = doc["id"]
        scores[doc_id] = scores.get(doc_id, 0) + 1.0 / (k + rank + 1)
        scores[doc_id + "_data"] = doc

    merged = []
    for key, score in scores.items():
        if key.endswith("_data"):
            continue
        data = scores.get(key + "_data", {})
        data["rrf_score"] = round(score, 6)
        merged.append(data)
    merged.sort(key=lambda x: x.get("rrf_score", 0), reverse=True)
    return merged


def process_one(raw):
    input_data = raw if isinstance(raw, dict) else (json.loads(raw) if str(raw).strip() else {})
    query = input_data.get("query", "")
    top_k = input_data.get("top_k", 3)

    # Search both sources
    tech_results = search_source(query, TECH_DOCS, "tech_docs")
    faq_results = search_source(query, FAQ, "faq")

    # Cross-rank fusion
    merged = reciprocal_rank_fusion(tech_results, faq_results)

    return ({
        "tech_docs": tech_results[:top_k], "faq": faq_results[:top_k], "merged": merged[:top_k],
        "tech_hits": len(tech_results),
        "faq_hits": len(faq_results),
        "strategy": "reciprocal_rank_fusion",
        "query": query
    })


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
    app = SidecarApp("rag_multi_source_search")
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
