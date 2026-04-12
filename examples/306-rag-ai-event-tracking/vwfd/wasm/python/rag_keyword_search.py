"""
RAG Keyword Article Search — Python WASM Module

Searches 12 support articles by keyword scoring.
Returns top-3 with relevance scores for LLM context.
"""

import json
import sys

ARTICLES = [
    {"id": 1, "title": "Getting Started", "tags": "setup install onboarding quickstart tutorial"},
    {"id": 2, "title": "Authentication", "tags": "login password oauth jwt token bearer api key"},
    {"id": 3, "title": "Billing & Invoices", "tags": "payment invoice billing subscription charge refund"},
    {"id": 4, "title": "API Rate Limits", "tags": "rate limit throttle 429 quota burst slow"},
    {"id": 5, "title": "Webhook Setup", "tags": "webhook callback event notification http post endpoint"},
    {"id": 6, "title": "Data Export", "tags": "export download csv json backup data migration"},
    {"id": 7, "title": "Team Management", "tags": "team invite member role permission admin user"},
    {"id": 8, "title": "SSO Configuration", "tags": "sso saml oidc single sign on enterprise identity"},
    {"id": 9, "title": "Troubleshooting", "tags": "error debug log trace issue problem fix"},
    {"id": 10, "title": "Mobile App", "tags": "mobile app ios android push notification offline"},
    {"id": 11, "title": "Security", "tags": "security audit compliance soc2 encryption tls certificate"},
    {"id": 12, "title": "Integrations", "tags": "integration connect slack teams jira github zapier"},
]


def search(query: str, top_k: int = 3) -> list:
    q_words = set(query.lower().split())
    scored = []
    for article in ARTICLES:
        tag_words = set(article["tags"].split())
        overlap = q_words & tag_words
        score = len(overlap) / max(len(q_words), 1)
        if score > 0:
            scored.append({
                "article_id": article["id"],
                "title": article["title"],
                "score": round(score, 3),
                "matched_terms": list(overlap)
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
        "total_articles": len(ARTICLES),
        "query": query
    }))


if __name__ == "__main__":
    main()
