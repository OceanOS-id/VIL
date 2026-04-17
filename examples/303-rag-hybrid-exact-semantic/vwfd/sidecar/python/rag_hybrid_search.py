"""
RAG Hybrid Search — Python WASM Module

Two-tier search:
  Tier 1: Exact match on FAQ triggers → instant answer (no LLM)
  Tier 2: BM25 keyword scoring → return top results for LLM augmentation
"""

import json
import math
import sys

EXACT_FAQS = {
    "how to reset password": "Go to Settings > Security > Reset Password. You'll receive a confirmation email.",
    "what is the pricing": "See our pricing page at /pricing. Plans start at $9/month.",
    "how to contact support": "Email support@example.com or use the chat widget on the bottom right.",
    "what payment methods": "We accept Visa, Mastercard, and bank transfer (IDR/USD).",
    "how to cancel subscription": "Go to Settings > Billing > Cancel. Your access continues until the period ends.",
}

KNOWLEDGE_ARTICLES = [
    {"id": "k1", "title": "Getting Started Guide", "content": "Create account sign up onboarding tutorial setup"},
    {"id": "k2", "title": "API Rate Limits", "content": "rate limit throttle 429 quota burst sustained"},
    {"id": "k3", "title": "Webhook Integration", "content": "webhook callback event notification HTTP POST payload"},
    {"id": "k4", "title": "Data Export", "content": "export download CSV JSON backup data migration"},
    {"id": "k5", "title": "Security & Compliance", "content": "security audit compliance SOC2 encryption TLS certificate"},
]


def exact_match(query: str) -> dict | None:
    q = query.lower().strip()
    for trigger, answer in EXACT_FAQS.items():
        if trigger in q:
            return {"matched": True, "answer": answer, "faq_id": trigger}
    return None


def bm25_score(query: str, content: str, k1: float = 1.5, b: float = 0.75) -> float:
    """Simplified BM25 scoring."""
    q_terms = set(query.lower().split())
    doc_terms = content.lower().split()
    doc_len = len(doc_terms)
    avg_dl = 8.0  # approximate average doc length

    score = 0.0
    for term in q_terms:
        tf = doc_terms.count(term)
        if tf > 0:
            idf = math.log(len(KNOWLEDGE_ARTICLES) / (1 + sum(1 for a in KNOWLEDGE_ARTICLES if term in a["content"].lower())))
            numerator = tf * (k1 + 1)
            denominator = tf + k1 * (1 - b + b * doc_len / avg_dl)
            score += idf * numerator / denominator
    return score


def keyword_search(query: str, top_k: int = 3) -> list:
    results = []
    for article in KNOWLEDGE_ARTICLES:
        score = bm25_score(query, article["content"])
        if score > 0:
            results.append({
                "id": article["id"],
                "title": article["title"],
                "score": round(score, 4),
                "method": "bm25"
            })
    results.sort(key=lambda x: x["score"], reverse=True)
    return results[:top_k]


def process_one(raw):
    input_data = raw if isinstance(raw, dict) else (json.loads(raw) if str(raw).strip() else {})
    query = input_data.get("query", "")

    # Tier 1: exact match
    exact = exact_match(query)
    if exact:
        return exact
        return

    # Tier 2: keyword search
    results = keyword_search(query)
    return ({
        "matched": False,
        "results": results,
        "method": "bm25_keyword",
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
    app = SidecarApp("rag_exact_match_check")
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
