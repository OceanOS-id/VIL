// ╔════════════════════════════════════════════════════════════╗
// ║  307 — Enterprise Internal Document Search                ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Enterprise / IT Policy Knowledge Base          ║
// ║  Pattern:  VX_APP                                         ║
// ║  Features: ShmSlice, ServiceCtx, VilResponse, VilModel,   ║
// ║            Collection, HnswConfig, QueryBuilder,          ║
// ║            SearchResult                                    ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Index 20+ IT policy documents in HNSW vector   ║
// ║  index, search by cosine similarity. Mock embeddings via  ║
// ║  word hashing into 64-dim vectors — no external model.    ║
// ╚════════════════════════════════════════════════════════════╝
//
// Run:   cargo run -p vil-rag-vectordb-knowledge-index
// Test:
//   curl http://localhost:3107/api/search/stats
//   curl -X POST -H "Content-Type: application/json" \
//     -d '{"query": "password reset policy", "top_k": 5}' \
//     http://localhost:3107/api/search/query
//   curl -X POST -H "Content-Type: application/json" \
//     -d '{"id": "custom-001", "title": "Custom Policy", "content": "All employees must ..."}' \
//     http://localhost:3107/api/search/index

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use vil_server::prelude::*;
use vil_vectordb::{Collection, HnswConfig, QueryBuilder, SearchResult};

const EMBEDDING_DIM: usize = 64;

// ── Request / Response models ───────────────────────────────────────

#[derive(Debug, Deserialize)]
struct IndexRequest {
    id: String,
    title: String,
    content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct IndexResponse {
    id: String,
    title: String,
    vector_id: u64,
    dimension: usize,
}

#[derive(Debug, Deserialize)]
struct QueryRequest {
    query: String,
    #[serde(default = "default_top_k")]
    top_k: usize,
    #[serde(default)]
    min_score: Option<f32>,
}

fn default_top_k() -> usize {
    5
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct QueryResponse {
    query: String,
    top_k: usize,
    results: Vec<DocResult>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct DocResult {
    id: u64,
    score: f32,
    doc_id: String,
    title: String,
    snippet: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct StatsResponse {
    collection: String,
    document_count: usize,
    dimension: usize,
}

// ── Mock embedding: hash words into a fixed 64-dim vector ───────────
// Deterministic: same text always produces the same vector.
// This gives real HNSW cosine similarity search without an actual model.

fn mock_embed(text: &str) -> Vec<f32> {
    let mut vec = vec![0.0f32; EMBEDDING_DIM];
    for word in text.split_whitespace() {
        let w = word.to_lowercase();
        let mut hasher = DefaultHasher::new();
        w.hash(&mut hasher);
        let h = hasher.finish();
        let idx = (h as usize) % EMBEDDING_DIM;
        vec[idx] += 1.0;
    }
    // L2-normalize so cosine similarity works well
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in &mut vec {
            *v /= norm;
        }
    }
    vec
}

// ── Shared state ────────────────────────────────────────────────────

struct AppState {
    collection: Collection,
}

// ── Handler: POST /api/search/index — ingest a document ─────────────

async fn index_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<IndexResponse>> {
    let req: IndexRequest = body.json()
        .map_err(|_| VilError::bad_request("invalid JSON — expected {id, title, content}"))?;

    if req.title.trim().is_empty() || req.content.trim().is_empty() {
        return Err(VilError::bad_request("title and content are required"));
    }

    let state = ctx.state::<Arc<AppState>>()
        .map_err(|_| VilError::internal("state not found"))?;

    let combined = format!("{} {}", req.title, req.content);
    let embedding = mock_embed(&combined);
    let dim = embedding.len();

    let metadata = serde_json::json!({
        "doc_id": req.id,
        "title": req.title,
    });

    let vector_id = state.collection.add(
        embedding,
        metadata,
        Some(req.content.chars().take(500).collect()),
    );

    Ok(VilResponse::ok(IndexResponse {
        id: req.id,
        title: req.title,
        vector_id,
        dimension: dim,
    }))
}

// ── Handler: POST /api/search/query — similarity search ─────────────

async fn query_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<QueryResponse>> {
    let req: QueryRequest = body.json()
        .map_err(|_| VilError::bad_request("invalid JSON — expected {query, top_k?}"))?;

    if req.query.trim().is_empty() {
        return Err(VilError::bad_request("query is required"));
    }

    let state = ctx.state::<Arc<AppState>>()
        .map_err(|_| VilError::internal("state not found"))?;

    let query_vec = mock_embed(&req.query);

    let results: Vec<SearchResult> = if let Some(min) = req.min_score {
        QueryBuilder::new(&state.collection)
            .vector(query_vec)
            .top_k(req.top_k)
            .min_score(min)
            .execute()
    } else {
        QueryBuilder::new(&state.collection)
            .vector(query_vec)
            .top_k(req.top_k)
            .execute()
    };

    let doc_results: Vec<DocResult> = results.into_iter().map(|r| {
        let doc_id = r.metadata.get("doc_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let title = r.metadata.get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("untitled")
            .to_string();
        let snippet = r.text.unwrap_or_default();
        DocResult {
            id: r.id,
            score: r.score,
            doc_id,
            title,
            snippet,
        }
    }).collect();

    Ok(VilResponse::ok(QueryResponse {
        query: req.query,
        top_k: req.top_k,
        results: doc_results,
    }))
}

// ── Handler: GET /api/search/stats — index stats ────────────────────

async fn stats_handler(ctx: ServiceCtx) -> HandlerResult<VilResponse<StatsResponse>> {
    let state = ctx.state::<Arc<AppState>>()
        .map_err(|_| VilError::internal("state not found"))?;

    Ok(VilResponse::ok(StatsResponse {
        collection: state.collection.name().to_string(),
        document_count: state.collection.count(),
        dimension: state.collection.dimension(),
    }))
}

// ── Seed data: 15+ IT policy documents ──────────────────────────────

fn seed_documents(collection: &Collection) {
    let docs: Vec<(&str, &str, &str)> = vec![
        ("POL-001", "Password Policy",
         "All employees must use passwords with minimum 12 characters, including uppercase, lowercase, digits, and special characters. Passwords must be rotated every 90 days. Reuse of the last 10 passwords is prohibited. Multi-factor authentication is required for all privileged accounts."),
        ("POL-002", "VPN Access Policy",
         "Remote access to the corporate network requires an approved VPN client with split tunneling disabled. Employees must connect through the VPN when accessing internal systems from outside the office. VPN sessions timeout after 8 hours of inactivity."),
        ("POL-003", "Email Usage Policy",
         "Corporate email must not be used for personal communication. All outgoing emails with attachments over 25MB must use the secure file transfer service. Suspicious emails should be reported to the security team immediately. Auto-forwarding to external addresses is prohibited."),
        ("POL-004", "Software Installation Policy",
         "Only IT-approved software may be installed on corporate devices. Employees must submit a software request through the IT service desk. Open source software requires security review before approval. All software must be kept up to date with the latest patches."),
        ("POL-005", "Multi-Factor Authentication Policy",
         "MFA is mandatory for all cloud services, VPN access, and administrative accounts. Approved MFA methods include hardware tokens, authenticator apps, and biometric verification. SMS-based authentication is deprecated and being phased out."),
        ("POL-006", "WiFi Network Policy",
         "The corporate WiFi network uses WPA3 enterprise authentication. Guest WiFi is isolated from the corporate network. Employees must not set up unauthorized wireless access points. WiFi passwords for guest access rotate weekly."),
        ("POL-007", "Printer and Scanning Policy",
         "Confidential documents must use secure print with badge release. Scanned documents containing sensitive data must be encrypted. Shared printers require user authentication. Print logs are retained for 90 days for audit purposes."),
        ("POL-008", "File Sharing Policy",
         "Corporate data must only be shared through approved platforms such as SharePoint and OneDrive. External file sharing requires manager approval. Public links with no expiry are prohibited. All shared files must have appropriate access controls."),
        ("POL-009", "Employee Onboarding IT Policy",
         "New employees receive IT equipment and accounts within their first day. Access provisioning follows the least privilege principle based on role. Mandatory security awareness training must be completed within the first week. All access is reviewed after the 90-day probation period."),
        ("POL-010", "Data Backup Policy",
         "Critical systems are backed up daily with incremental backups. Full backups run weekly and are stored off-site. Backup restoration tests are performed quarterly. Retention period is 7 years for financial data and 3 years for operational data."),
        ("POL-011", "Information Security Policy",
         "All employees are responsible for protecting company information assets. Security incidents must be reported within 1 hour of discovery. Annual penetration testing is mandatory. Security patches must be applied within 72 hours of release for critical vulnerabilities."),
        ("POL-012", "Regulatory Compliance Policy",
         "The organization complies with GDPR, SOX, and industry-specific regulations. Data processing activities must be documented in the processing register. Privacy impact assessments are required for new systems handling personal data. Annual compliance audits are conducted by external auditors."),
        ("POL-013", "Data Classification Policy",
         "Data is classified into four levels: Public, Internal, Confidential, and Restricted. All documents must be labeled with their classification level. Restricted data requires encryption at rest and in transit. Access to confidential data requires explicit authorization from the data owner."),
        ("POL-014", "Incident Response Policy",
         "The incident response team must be activated within 30 minutes of a confirmed security incident. All incidents are categorized by severity: Critical, High, Medium, Low. Post-incident reviews must be completed within 5 business days. Root cause analysis reports are shared with senior management."),
        ("POL-015", "Change Management Policy",
         "All changes to production systems must follow the change management process. Emergency changes require retrospective approval within 24 hours. Change windows are Tuesday and Thursday 10PM-2AM. Rollback procedures must be documented before any change is approved."),
        ("POL-016", "BYOD Policy",
         "Personal devices accessing corporate data must be enrolled in mobile device management. Minimum OS versions are enforced: iOS 16+, Android 13+. Remote wipe capability must be enabled. Corporate data containers must use encryption and separate from personal data."),
        ("POL-017", "Cloud Services Policy",
         "All cloud services must be approved by IT security before adoption. Data residency requirements must be verified for each cloud provider. Cloud services storing sensitive data require SOC2 Type II certification. Shadow IT usage is monitored and reported quarterly."),
        ("POL-018", "Network Segmentation Policy",
         "The corporate network is segmented into zones: DMZ, internal, development, and production. Inter-zone traffic requires firewall rules approved by the security team. IoT devices are isolated in a dedicated network segment. Network segmentation is verified through quarterly audits."),
    ];

    for (doc_id, title, content) in &docs {
        let combined = format!("{} {}", title, content);
        let embedding = mock_embed(&combined);
        let metadata = serde_json::json!({
            "doc_id": doc_id,
            "title": title,
        });
        collection.add(embedding, metadata, Some(content.to_string()));
    }
}

// ── Main ────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  307 — Enterprise Internal Document Search                  ║");
    println!("║  VectorDB: HNSW index | 64-dim mock embeddings             ║");
    println!("║  Pre-seeded: 18 IT policy documents                        ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("  Index:  POST http://localhost:3107/api/search/index");
    println!("  Query:  POST http://localhost:3107/api/search/query");
    println!("  Stats:  GET  http://localhost:3107/api/search/stats");
    println!();

    let collection = Collection::new("it-policies", EMBEDDING_DIM, HnswConfig::default());

    // Pre-seed IT policy documents
    seed_documents(&collection);
    println!("  Seeded {} documents into HNSW index", collection.count());
    println!();

    let app_state = Arc::new(AppState { collection });

    let svc = ServiceProcess::new("search")
        .prefix("/api/search")
        .endpoint(Method::POST, "/index", post(index_handler))
        .endpoint(Method::POST, "/query", post(query_handler))
        .endpoint(Method::GET,  "/stats", get(stats_handler))
        .state(app_state);

    VilApp::new("rag-vectordb-knowledge-index")
        .port(3107)
        .observer(true)
        .service(svc)
        .run()
        .await;
}
