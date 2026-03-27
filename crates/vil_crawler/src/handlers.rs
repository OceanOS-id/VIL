//! HTTP handlers for the crawler plugin — wired to real CrawlConfig state.

use vil_server::prelude::*;
use std::sync::Arc;

use crate::config::CrawlConfig;
use crate::crawler::Crawler;

// ── Request / Response types ────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CrawlRequest {
    pub url: String,
    #[serde(default = "default_max_pages")]
    pub max_pages: usize,
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
}

fn default_max_pages() -> usize { 10 }
fn default_max_depth() -> usize { 2 }

#[derive(Debug, Serialize)]
pub struct CrawlResponseBody {
    pub pages_crawled: usize,
    pub results: Vec<CrawlPageSummary>,
}

#[derive(Debug, Serialize)]
pub struct CrawlPageSummary {
    pub url: String,
    pub status: u16,
    pub title: Option<String>,
    pub text_length: usize,
    pub links_found: usize,
    pub crawl_time_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CrawlerStatsBody {
    pub max_pages: usize,
    pub max_depth: usize,
    pub concurrency: usize,
    pub timeout_ms: u64,
    pub respect_robots: bool,
    pub allowed_domains: Vec<String>,
}

// ── Handlers ────────────────────────────────────────────────────────

/// POST /crawl — Crawl a website starting from the given URL.
pub async fn crawl_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<CrawlResponseBody>> {
    let config = ctx.state::<Arc<CrawlConfig>>()?;
    let req: CrawlRequest = body.json().map_err(|e| VilError::bad_request(e.to_string()))?;
    if req.url.trim().is_empty() {
        return Err(VilError::bad_request("url must not be empty"));
    }

    let crawl_config = CrawlConfig {
        max_pages: req.max_pages,
        max_depth: req.max_depth,
        concurrency: config.concurrency,
        timeout_ms: config.timeout_ms,
        respect_robots: config.respect_robots,
        allowed_domains: config.allowed_domains.clone(),
    };

    let crawler = Crawler::new(crawl_config);
    let results = crawler.crawl_site(&req.url).await;

    let summaries: Vec<CrawlPageSummary> = results
        .iter()
        .map(|r| CrawlPageSummary {
            url: r.url.clone(),
            status: r.status,
            title: r.title.clone(),
            text_length: r.text.len(),
            links_found: r.links.len(),
            crawl_time_ms: r.crawl_time_ms,
        })
        .collect();

    Ok(VilResponse::ok(CrawlResponseBody {
        pages_crawled: summaries.len(),
        results: summaries,
    }))
}

/// GET /stats — return real crawler configuration.
pub async fn stats_handler(
    ctx: ServiceCtx,
) -> HandlerResult<VilResponse<CrawlerStatsBody>> {
    let config = ctx.state::<Arc<CrawlConfig>>()?;
    Ok(VilResponse::ok(CrawlerStatsBody {
        max_pages: config.max_pages,
        max_depth: config.max_depth,
        concurrency: config.concurrency,
        timeout_ms: config.timeout_ms,
        respect_robots: config.respect_robots,
        allowed_domains: config.allowed_domains.clone(),
    }))
}
