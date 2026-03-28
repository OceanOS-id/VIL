use serde::{Deserialize, Serialize};

/// Configuration for a crawl session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlConfig {
    /// Maximum number of pages to crawl.
    pub max_pages: usize,
    /// Maximum link-follow depth from the start URL.
    pub max_depth: usize,
    /// Number of concurrent fetch tasks.
    pub concurrency: usize,
    /// Per-request timeout in milliseconds.
    pub timeout_ms: u64,
    /// Whether to respect robots.txt directives.
    pub respect_robots: bool,
    /// If non-empty, only crawl URLs whose host is in this list.
    pub allowed_domains: Vec<String>,
}

impl Default for CrawlConfig {
    fn default() -> Self {
        Self {
            max_pages: 100,
            max_depth: 3,
            concurrency: 4,
            timeout_ms: 10_000,
            respect_robots: true,
            allowed_domains: Vec::new(),
        }
    }
}

impl CrawlConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn max_pages(mut self, n: usize) -> Self {
        self.max_pages = n;
        self
    }

    pub fn max_depth(mut self, d: usize) -> Self {
        self.max_depth = d;
        self
    }

    pub fn concurrency(mut self, c: usize) -> Self {
        self.concurrency = c;
        self
    }

    pub fn timeout_ms(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    pub fn respect_robots(mut self, r: bool) -> Self {
        self.respect_robots = r;
        self
    }

    pub fn allowed_domains(mut self, domains: Vec<String>) -> Self {
        self.allowed_domains = domains;
        self
    }

    /// Check whether a given host is allowed by the config.
    pub fn is_domain_allowed(&self, host: &str) -> bool {
        if self.allowed_domains.is_empty() {
            return true;
        }
        self.allowed_domains
            .iter()
            .any(|d| host.ends_with(d.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let cfg = CrawlConfig::new()
            .max_pages(50)
            .max_depth(2)
            .concurrency(8)
            .timeout_ms(5000)
            .respect_robots(false)
            .allowed_domains(vec!["example.com".into()]);

        assert_eq!(cfg.max_pages, 50);
        assert_eq!(cfg.max_depth, 2);
        assert_eq!(cfg.concurrency, 8);
        assert_eq!(cfg.timeout_ms, 5000);
        assert!(!cfg.respect_robots);
        assert_eq!(cfg.allowed_domains, vec!["example.com".to_string()]);
    }

    #[test]
    fn test_domain_allowed_empty() {
        let cfg = CrawlConfig::new();
        assert!(cfg.is_domain_allowed("anything.com"));
    }

    #[test]
    fn test_domain_allowed_filter() {
        let cfg = CrawlConfig::new().allowed_domains(vec!["example.com".into(), "test.org".into()]);
        assert!(cfg.is_domain_allowed("example.com"));
        assert!(cfg.is_domain_allowed("sub.example.com"));
        assert!(cfg.is_domain_allowed("test.org"));
        assert!(!cfg.is_domain_allowed("other.com"));
    }
}
