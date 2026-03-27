// ── N04: Benchmark Report ───────────────────────────────────────────
use serde::{Deserialize, Serialize};

/// Result for a single benchmark.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchResult {
    pub benchmark: String,
    pub score: f32,
    pub cases_passed: usize,
    pub cases_total: usize,
}

/// Aggregated report across all benchmarks in a suite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchReport {
    pub results: Vec<BenchResult>,
    pub overall_score: f32,
}

impl BenchReport {
    pub fn new(results: Vec<BenchResult>) -> Self {
        let overall_score = if results.is_empty() {
            0.0
        } else {
            let sum: f32 = results.iter().map(|r| r.score).sum();
            sum / results.len() as f32
        };
        Self {
            results,
            overall_score,
        }
    }

    /// Number of benchmarks in the report.
    pub fn benchmark_count(&self) -> usize {
        self.results.len()
    }

    /// Total cases passed across all benchmarks.
    pub fn total_passed(&self) -> usize {
        self.results.iter().map(|r| r.cases_passed).sum()
    }

    /// Total cases across all benchmarks.
    pub fn total_cases(&self) -> usize {
        self.results.iter().map(|r| r.cases_total).sum()
    }

    /// Serialize to JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_aggregation() {
        let results = vec![
            BenchResult { benchmark: "math".into(), score: 0.8, cases_passed: 4, cases_total: 5 },
            BenchResult { benchmark: "logic".into(), score: 0.6, cases_passed: 3, cases_total: 5 },
        ];
        let report = BenchReport::new(results);
        assert!((report.overall_score - 0.7).abs() < 0.01);
        assert_eq!(report.total_passed(), 7);
        assert_eq!(report.total_cases(), 10);
    }

    #[test]
    fn empty_report() {
        let report = BenchReport::new(vec![]);
        assert_eq!(report.overall_score, 0.0);
        assert_eq!(report.benchmark_count(), 0);
    }
}
