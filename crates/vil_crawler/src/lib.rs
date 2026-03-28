//! # VIL Web Crawler (I01)
//!
//! Async concurrent web crawler with BFS traversal, robots.txt support,
//! domain filtering, and depth limiting.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use vil_crawler::{Crawler, CrawlConfig};
//!
//! # async fn example() {
//! let config = CrawlConfig::new()
//!     .max_pages(10)
//!     .max_depth(2)
//!     .concurrency(4);
//!
//! let crawler = Crawler::new(config);
//! let results = crawler.crawl_site("https://example.com").await;
//! for r in &results {
//!     println!("{} — {} chars", r.url, r.text.len());
//! }
//! # }
//! ```

pub mod config;
pub mod crawler;
pub mod handlers;
pub mod pipeline_sse;
pub mod plugin;
pub mod result;
pub mod robots;
pub mod semantic;

pub use config::CrawlConfig;
pub use crawler::{CrawlError, Crawler};
pub use plugin::CrawlerPlugin;
pub use result::CrawlResult;
pub use robots::RobotsChecker;
pub use semantic::{CrawlEvent, CrawlFault, CrawlFaultType, CrawlerState};
