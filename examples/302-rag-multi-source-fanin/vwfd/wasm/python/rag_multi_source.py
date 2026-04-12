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


def main():
    raw = sys.stdin.read()
    input_data = json.loads(raw) if raw.strip() else {}
    query = input_data.get("query", "")
    top_k = input_data.get("top_k", 3)

    # Search both sources
    tech_results = search_source(query, TECH_DOCS, "tech_docs")
    faq_results = search_source(query, FAQ, "faq")

    # Cross-rank fusion
    merged = reciprocal_rank_fusion(tech_results, faq_results)

    print(json.dumps({
        "merged": merged[:top_k],
        "tech_hits": len(tech_results),
        "faq_hits": len(faq_results),
        "strategy": "reciprocal_rank_fusion",
        "query": query
    }))


if __name__ == "__main__":
    main()
