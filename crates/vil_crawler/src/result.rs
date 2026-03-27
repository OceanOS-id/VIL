use serde::{Deserialize, Serialize};

/// Result of crawling a single URL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlResult {
    /// The URL that was fetched.
    pub url: String,
    /// HTTP status code.
    pub status: u16,
    /// Extracted plain text (HTML tags stripped).
    pub text: String,
    /// Page title extracted from `<title>`.
    pub title: Option<String>,
    /// Links found on the page.
    pub links: Vec<String>,
    /// Content-Type header value.
    pub content_type: Option<String>,
    /// Time taken to crawl this page in milliseconds.
    pub crawl_time_ms: u64,
}

impl CrawlResult {
    /// Whether the fetch was successful (2xx status).
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

/// Strip HTML tags from text, returning plain text.
pub fn strip_html_tags(html: &str) -> String {
    let re = regex::Regex::new(r"<[^>]+>").expect("valid regex");
    let text = re.replace_all(html, " ");
    // Collapse whitespace
    let ws = regex::Regex::new(r"\s+").expect("valid regex");
    ws.replace_all(text.trim(), " ").to_string()
}

/// Extract the `<title>` content from HTML.
pub fn extract_title(html: &str) -> Option<String> {
    let re = regex::Regex::new(r"(?i)<title[^>]*>(.*?)</title>").ok()?;
    re.captures(html).map(|c| c[1].trim().to_string())
}

/// Extract all `href` links from HTML.
pub fn extract_links(html: &str, base_url: &str) -> Vec<String> {
    let re = regex::Regex::new(r#"href\s*=\s*["']([^"']+)["']"#).expect("valid regex");
    re.captures_iter(html)
        .filter_map(|cap| {
            let href = cap[1].trim();
            if href.starts_with("http://") || href.starts_with("https://") {
                Some(href.to_string())
            } else if href.starts_with('/') {
                // Resolve relative to base
                let base = base_url.trim_end_matches('/');
                // Extract scheme + host from base
                if let Some(idx) = base.find("://") {
                    let rest = &base[idx + 3..];
                    if let Some(slash) = rest.find('/') {
                        let origin = &base[..idx + 3 + slash];
                        Some(format!("{}{}", origin, href))
                    } else {
                        Some(format!("{}{}", base, href))
                    }
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_html_tags() {
        let html = "<html><body><h1>Hello</h1><p>World</p></body></html>";
        let text = strip_html_tags(html);
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));
        assert!(!text.contains("<h1>"));
    }

    #[test]
    fn test_extract_title() {
        let html = "<html><head><title>My Page</title></head><body></body></html>";
        assert_eq!(extract_title(html), Some("My Page".to_string()));
    }

    #[test]
    fn test_extract_title_missing() {
        let html = "<html><head></head><body></body></html>";
        assert_eq!(extract_title(html), None);
    }

    #[test]
    fn test_extract_links() {
        let html = r#"<a href="https://example.com/a">A</a><a href="/b">B</a>"#;
        let links = extract_links(html, "https://example.com/page");
        assert!(links.contains(&"https://example.com/a".to_string()));
        assert!(links.contains(&"https://example.com/b".to_string()));
    }

    #[test]
    fn test_crawl_result_success() {
        let r = CrawlResult {
            url: "https://example.com".into(),
            status: 200,
            text: "Hello".into(),
            title: Some("Test".into()),
            links: vec![],
            content_type: Some("text/html".into()),
            crawl_time_ms: 42,
        };
        assert!(r.is_success());

        let r2 = CrawlResult { status: 404, ..r };
        assert!(!r2.is_success());
    }
}
