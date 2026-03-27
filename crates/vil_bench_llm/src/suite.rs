// ── N04: Benchmark Suite ────────────────────────────────────────────
use crate::benchmark::Benchmark;
use crate::report::{BenchReport, BenchResult};

/// A suite that runs multiple benchmarks and aggregates results.
pub struct BenchSuite {
    pub benchmarks: Vec<Box<dyn Benchmark>>,
}

impl BenchSuite {
    pub fn new() -> Self {
        Self {
            benchmarks: Vec::new(),
        }
    }

    pub fn add(mut self, bench: Box<dyn Benchmark>) -> Self {
        self.benchmarks.push(bench);
        self
    }

    /// Run all benchmarks using a scoring function that provides model answers.
    /// `answer_fn` takes a question and returns the model's answer.
    pub fn run<F>(&self, answer_fn: F) -> BenchReport
    where
        F: Fn(&str) -> String,
    {
        let results: Vec<BenchResult> = self
            .benchmarks
            .iter()
            .map(|bench| {
                let cases = bench.cases();
                let total = cases.len();
                let mut passed = 0usize;
                let mut score_sum = 0.0f32;

                for case in &cases {
                    let answer = answer_fn(&case.question);
                    let score = bench.evaluate(&answer, &case.expected_answer);
                    score_sum += score;
                    if score >= 0.5 {
                        passed += 1;
                    }
                }

                let avg_score = if total > 0 {
                    score_sum / total as f32
                } else {
                    0.0
                };

                BenchResult {
                    benchmark: bench.name().to_string(),
                    score: avg_score,
                    cases_passed: passed,
                    cases_total: total,
                }
            })
            .collect();

        BenchReport::new(results)
    }
}

impl Default for BenchSuite {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::built_in::{FactBench, LogicBench, MathBench};

    #[test]
    fn suite_runs_all_benchmarks() {
        let suite = BenchSuite::new()
            .add(Box::new(MathBench))
            .add(Box::new(LogicBench))
            .add(Box::new(FactBench));

        // Perfect oracle answer function
        let report = suite.run(|q| {
            if q.contains("2 + 3") { "5".into() }
            else if q.contains("10 * 7") { "70".into() }
            else if q.contains("144 / 12") { "12".into() }
            else if q.contains("1000 - 357") { "643".into() }
            else if q.contains("15 + 28") { "43".into() }
            else if q.contains("Whiskers") { "Yes".into() }
            else if q.contains("necessarily raining") { "No".into() }
            else if q.contains("tallest") { "A".into() }
            else if q.contains("squares") { "True".into() }
            else if q.contains("capital of France") { "Paris".into() }
            else if q.contains("closest to the Sun") { "Mercury".into() }
            else if q.contains("chemical symbol") { "H2O".into() }
            else if q.contains("Romeo") { "Shakespeare".into() }
            else { "unknown".into() }
        });

        assert_eq!(report.benchmark_count(), 3);
        assert!(report.overall_score > 0.9);
    }

    #[test]
    fn empty_suite() {
        let suite = BenchSuite::new();
        let report = suite.run(|_| "answer".into());
        assert_eq!(report.benchmark_count(), 0);
        assert_eq!(report.overall_score, 0.0);
    }
}
