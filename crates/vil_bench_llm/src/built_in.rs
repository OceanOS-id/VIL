// ── N04: Built-in Benchmarks ────────────────────────────────────────
use crate::benchmark::{BenchCase, Benchmark};

// ── MathBench ───────────────────────────────────────────────────────

/// Simple arithmetic benchmark.
pub struct MathBench;

impl Benchmark for MathBench {
    fn name(&self) -> &str {
        "math"
    }

    fn cases(&self) -> Vec<BenchCase> {
        vec![
            BenchCase::new("What is 2 + 3?", "5", "arithmetic"),
            BenchCase::new("What is 10 * 7?", "70", "arithmetic"),
            BenchCase::new("What is 144 / 12?", "12", "arithmetic"),
            BenchCase::new("What is 1000 - 357?", "643", "arithmetic"),
            BenchCase::new("What is 15 + 28?", "43", "arithmetic"),
        ]
    }

    fn evaluate(&self, answer: &str, expected: &str) -> f32 {
        let answer_clean = answer.trim().to_lowercase();
        let expected_clean = expected.trim().to_lowercase();
        if answer_clean.contains(&expected_clean) {
            1.0
        } else {
            0.0
        }
    }
}

// ── LogicBench ──────────────────────────────────────────────────────

/// Simple logic / reasoning benchmark.
pub struct LogicBench;

impl Benchmark for LogicBench {
    fn name(&self) -> &str {
        "logic"
    }

    fn cases(&self) -> Vec<BenchCase> {
        vec![
            BenchCase::new(
                "If all cats are animals and Whiskers is a cat, is Whiskers an animal?",
                "yes",
                "deduction",
            ),
            BenchCase::new(
                "If it is raining then the ground is wet. The ground is wet. Is it necessarily raining?",
                "no",
                "fallacy",
            ),
            BenchCase::new(
                "A is taller than B. B is taller than C. Who is the tallest?",
                "a",
                "ordering",
            ),
            BenchCase::new(
                "True or False: All squares are rectangles.",
                "true",
                "geometry_logic",
            ),
        ]
    }

    fn evaluate(&self, answer: &str, expected: &str) -> f32 {
        let answer_clean = answer.trim().to_lowercase();
        let expected_clean = expected.trim().to_lowercase();
        if answer_clean.contains(&expected_clean) {
            1.0
        } else {
            0.0
        }
    }
}

// ── FactBench ───────────────────────────────────────────────────────

/// Factual Q&A benchmark.
pub struct FactBench;

impl Benchmark for FactBench {
    fn name(&self) -> &str {
        "factual"
    }

    fn cases(&self) -> Vec<BenchCase> {
        vec![
            BenchCase::new("What is the capital of France?", "paris", "geography"),
            BenchCase::new("What planet is closest to the Sun?", "mercury", "astronomy"),
            BenchCase::new("What is the chemical symbol for water?", "h2o", "chemistry"),
            BenchCase::new("Who wrote Romeo and Juliet?", "shakespeare", "literature"),
        ]
    }

    fn evaluate(&self, answer: &str, expected: &str) -> f32 {
        let answer_clean = answer.trim().to_lowercase();
        let expected_clean = expected.trim().to_lowercase();
        if answer_clean.contains(&expected_clean) {
            1.0
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn math_bench_correct() {
        let b = MathBench;
        assert_eq!(b.evaluate("The answer is 5", "5"), 1.0);
    }

    #[test]
    fn math_bench_incorrect() {
        let b = MathBench;
        assert_eq!(b.evaluate("The answer is 6", "5"), 0.0);
    }

    #[test]
    fn logic_bench_cases_non_empty() {
        let b = LogicBench;
        assert!(!b.cases().is_empty());
    }

    #[test]
    fn logic_bench_correct() {
        let b = LogicBench;
        assert_eq!(b.evaluate("Yes, Whiskers is an animal.", "yes"), 1.0);
    }

    #[test]
    fn fact_bench_correct() {
        let b = FactBench;
        assert_eq!(b.evaluate("Paris is the capital.", "paris"), 1.0);
    }

    #[test]
    fn fact_bench_case_insensitive() {
        let b = FactBench;
        assert_eq!(b.evaluate("MERCURY", "mercury"), 1.0);
    }
}
