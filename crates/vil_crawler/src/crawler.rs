use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use dashmap::DashMap;
use tokio::sync::Semaphore;
use vil_log::app_log;

use crate::config::CrawlConfig;
use crate::result::{extract_links, extract_title, strip_html_tags, CrawlResult};
use crate::robots::RobotsChecker;

/// Async web crawler with concurrent BFS traversal.
pub struct Crawler {
    client: reqwest::Client,
    config: CrawlConfig,
    visited: Arc<DashMap<String, bool>>,
}

impl Crawler {
    /// Create a new crawler with the given config.
    pub fn new(config: CrawlConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms))
            .user_agent("vil-crawler/0.1")
            .build()
            .expect("failed to build HTTP client");

        Self {
            client,
            config,
            visited: Arc::new(DashMap::new()),
        }
    }

    /// Create a crawler with default config.
    pub fn default_crawler() -> Self {
        Self::new(CrawlConfig::default())
    }

    /// Crawl a single URL and return the result.
    /// Rejects URLs targeting private/internal IP ranges to prevent SSRF.
    pub async fn crawl_url(&self, url: &str) -> Result<CrawlResult, CrawlError> {
        if is_private_url(url) {
            return Err(CrawlError::FetchError(
                "URL targets a private/internal address — blocked for security".into(),
            ));
        }

        let start = Instant::now();

        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| CrawlError::FetchError(e.to_string()))?;

        let status = resp.status().as_u16();
        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let body = resp
            .text()
            .await
            .map_err(|e| CrawlError::FetchError(e.to_string()))?;

        let title = extract_title(&body);
        let links = extract_links(&body, url);
        let text = strip_html_tags(&body);
        let crawl_time_ms = start.elapsed().as_millis() as u64;

        Ok(CrawlResult {
            url: url.to_string(),
            status,
            text,
            title,
            links,
            content_type,
            crawl_time_ms,
        })
    }

    /// BFS crawl starting from `start_url`, respecting config limits.
    pub async fn crawl_site(&self, start_url: &str) -> Vec<CrawlResult> {
        let mut results = Vec::new();
        let page_count = Arc::new(AtomicUsize::new(0));
        let semaphore = Arc::new(Semaphore::new(self.config.concurrency));

        // Optionally fetch robots.txt
        let robots = if self.config.respect_robots {
            self.fetch_robots(start_url).await
        } else {
            None
        };

        // BFS queue: (url, depth)
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();
        queue.push_back((start_url.to_string(), 0));
        self.visited.insert(start_url.to_string(), true);

        while let Some((url, depth)) = queue.pop_front() {
            if page_count.load(Ordering::Relaxed) >= self.config.max_pages {
                break;
            }

            // Check domain allowance
            if let Some(host) = extract_host(&url) {
                if !self.config.is_domain_allowed(&host) {
                    app_log!(Debug, "crawler_skip", { url: url.clone(), reason: "domain_not_allowed" });
                    continue;
                }
            }

            // Check robots.txt
            if let Some(ref checker) = robots {
                if let Some(path) = extract_path(&url) {
                    if !checker.is_allowed("vil-crawler", &path) {
                        app_log!(Debug, "crawler_skip", { url: url.clone(), reason: "robots_disallowed" });
                        continue;
                    }
                }
            }

            let _permit = semaphore
                .clone()
                .acquire_owned()
                .await
                .expect("semaphore closed");

            match self.crawl_url(&url).await {
                Ok(result) => {
                    page_count.fetch_add(1, Ordering::Relaxed);

                    // Enqueue discovered links if we haven't hit depth limit
                    if depth < self.config.max_depth {
                        for link in &result.links {
                            if !self.visited.contains_key(link) {
                                self.visited.insert(link.clone(), true);
                                queue.push_back((link.clone(), depth + 1));
                            }
                        }
                    }

                    results.push(result);
                }
                Err(e) => {
                    app_log!(Warn, "crawler_failed", { url: url.clone(), error: e.to_string() });
                }
            }
        }

        results
    }

    /// Fetch and parse robots.txt for the given URL's origin.
    async fn fetch_robots(&self, url: &str) -> Option<RobotsChecker> {
        let origin = extract_origin(url)?;
        let robots_url = format!("{}/robots.txt", origin);
        let resp = self.client.get(&robots_url).send().await.ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let body = resp.text().await.ok()?;
        Some(RobotsChecker::parse(&body))
    }
}

/// Check if a URL targets a private/internal IP range (SSRF prevention).
fn is_private_url(url: &str) -> bool {
    let host = match url.find("://") {
        Some(idx) => {
            let rest = &url[idx + 3..];
            rest.split('/')
                .next()
                .unwrap_or("")
                .split(':')
                .next()
                .unwrap_or("")
        }
        None => return true,
    };
    if matches!(
        host,
        "localhost"
            | "0.0.0.0"
            | "[::]"
            | "[::1]"
            | "metadata.google.internal"
            | "metadata.internal"
    ) {
        return true;
    }
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        return match ip {
            std::net::IpAddr::V4(v4) => {
                v4.is_loopback()
                    || v4.is_private()
                    || v4.is_link_local()
                    || v4.is_broadcast()
                    || v4.is_unspecified()
                    || v4.octets()[0] == 100 && (v4.octets()[1] & 0xC0) == 64
            }
            std::net::IpAddr::V6(v6) => v6.is_loopback() || v6.is_unspecified(),
        };
    }
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return true;
    }
    false
}

/// Extract the origin (scheme + host) from a URL.
fn extract_origin(url: &str) -> Option<String> {
    let idx = url.find("://")?;
    let rest = &url[idx + 3..];
    if let Some(slash) = rest.find('/') {
        Some(url[..idx + 3 + slash].to_string())
    } else {
        Some(url.to_string())
    }
}

/// Extract the host from a URL.
fn extract_host(url: &str) -> Option<String> {
    let idx = url.find("://")?;
    let rest = &url[idx + 3..];
    let host = rest.split('/').next()?;
    // Strip port
    let host = host.split(':').next()?;
    Some(host.to_string())
}

/// Extract the path from a URL.
fn extract_path(url: &str) -> Option<String> {
    let idx = url.find("://")?;
    let rest = &url[idx + 3..];
    if let Some(slash) = rest.find('/') {
        Some(rest[slash..].to_string())
    } else {
        Some("/".to_string())
    }
}

/// Errors that can occur during crawling.
#[derive(Debug, Clone)]
pub enum CrawlError {
    FetchError(String),
    Timeout,
    RobotsDisallowed,
}

impl std::fmt::Display for CrawlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CrawlError::FetchError(e) => write!(f, "fetch error: {}", e),
            CrawlError::Timeout => write!(f, "request timed out"),
            CrawlError::RobotsDisallowed => write!(f, "disallowed by robots.txt"),
        }
    }
}

impl std::error::Error for CrawlError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_origin() {
        assert_eq!(
            extract_origin("https://example.com/path/page"),
            Some("https://example.com".to_string())
        );
        assert_eq!(
            extract_origin("http://localhost:8080/api"),
            Some("http://localhost:8080".to_string())
        );
    }

    #[test]
    fn test_extract_host() {
        assert_eq!(
            extract_host("https://example.com/path"),
            Some("example.com".to_string())
        );
        assert_eq!(
            extract_host("https://sub.example.com:443/path"),
            Some("sub.example.com".to_string())
        );
    }

    #[test]
    fn test_extract_path() {
        assert_eq!(
            extract_path("https://example.com/api/v1"),
            Some("/api/v1".to_string())
        );
        assert_eq!(extract_path("https://example.com"), Some("/".to_string()));
    }
}
