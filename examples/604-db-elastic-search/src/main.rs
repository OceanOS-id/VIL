// =============================================================================
// example-604-db-elastic-search — Elasticsearch index + search with db_log!
// =============================================================================
//
// Demonstrates:
//   - ElasticClient::new() with a local Elasticsearch config
//   - index (insert a document) and search
//   - db_log! auto-emitted by vil_db_elastic on every operation
//   - StdoutDrain::resolved() output
//
// Requires: Elasticsearch running locally.
// Quick start:
//   docker run -p 9200:9200 -e discovery.type=single-node \
//     -e xpack.security.enabled=false elasticsearch:8.13.0
//
// Without Docker, this example prints config and exits gracefully.
// =============================================================================

use serde_json::json;
use vil_db_elastic::{ElasticClient, ElasticConfig};
use vil_log::drain::{StdoutDrain, StdoutFormat};
use vil_log::runtime::init_logging;
use vil_log::{LogConfig, LogLevel};

#[tokio::main]
async fn main() {
    // ── Init vil_log with resolved drain ──
    let config = LogConfig {
        ring_slots:        4096,
        level:             LogLevel::Info,
        batch_size:        64,
        flush_interval_ms: 50,
        threads:           None,
    };
    let _task = init_logging(config, StdoutDrain::new(StdoutFormat::Resolved));

    println!();
    println!("  example-604-db-elastic-search");
    println!("  Elasticsearch index + search with db_log! auto-emit");
    println!();

    let es_cfg = ElasticConfig {
        url:      "http://localhost:9200".into(),
        username: None,
        password: None,
    };

    println!("  Connecting to Elasticsearch: {}", es_cfg.url);
    println!();
    println!("  NOTE: Requires Elasticsearch running locally.");
    println!("  Start with:");
    println!("    docker run -p 9200:9200 \\");
    println!("      -e discovery.type=single-node \\");
    println!("      -e xpack.security.enabled=false \\");
    println!("      elasticsearch:8.13.0");
    println!();

    let client = match ElasticClient::new(es_cfg) {
        Ok(c)  => c,
        Err(e) => {
            println!("  [SKIP] Cannot build Elasticsearch client: {:?}", e);
            return;
        }
    };

    // ── INDEX (insert) ──
    let doc = json!({
        "title":    "VIL Framework Introduction",
        "category": "technology",
        "views":    1024u32,
        "published": true
    });

    match client.index("vil-articles", "article-1", doc).await {
        Ok(res) => println!("  INDEX  vil-articles  id={}  result={}", res.id, res.result),
        Err(e)  => {
            println!("  INDEX  error: {:?}", e);
            println!("  [SKIP] Elasticsearch not reachable.");
            return;
        }
    }

    // Allow indexing to settle
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // ── SEARCH ──
    let query = json!({
        "query": {
            "match": { "category": "technology" }
        }
    });

    match client.search("vil-articles", query).await {
        Ok(res) => {
            println!("  SEARCH vil-articles  total={}  hits={}", res.total, res.hits.len());
            for hit in &res.hits {
                if let Some(src) = hit.get("_source") {
                    println!("         - {}", src.get("title").and_then(|t| t.as_str()).unwrap_or("?"));
                }
            }
        }
        Err(e) => println!("  SEARCH error: {:?}", e),
    }

    // Allow drain to flush
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    println!();
    println!("  Done. db_log! entries emitted above in resolved format.");
    println!();
}
