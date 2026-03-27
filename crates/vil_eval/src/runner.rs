//! EvalRunner — batch evaluation runner.

use crate::dataset::EvalDataset;
use crate::evaluator::EvalMetric;
use crate::report::{CaseResult, EvalReport};

/// Runs evaluation metrics over a dataset.
pub struct EvalRunner {
    /// Metrics to evaluate.
    pub metrics: Vec<Box<dyn EvalMetric>>,
    /// The dataset to evaluate.
    pub dataset: EvalDataset,
}

impl EvalRunner {
    /// Create a new runner.
    pub fn new(dataset: EvalDataset) -> Self {
        Self {
            metrics: Vec::new(),
            dataset,
        }
    }

    /// Add a metric.
    pub fn add_metric(mut self, metric: Box<dyn EvalMetric>) -> Self {
        self.metrics.push(metric);
        self
    }

    /// Run all metrics over all cases and produce a report.
    pub fn run(&self) -> EvalReport {
        let mut report = EvalReport::new();

        for (idx, case) in self.dataset.cases.iter().enumerate() {
            let mut scores = Vec::new();
            for metric in &self.metrics {
                let score = metric.evaluate(
                    &case.question,
                    &case.answer,
                    &case.context,
                    case.reference.as_deref(),
                );
                scores.push(score);
            }
            report.results.push(CaseResult {
                case_index: idx,
                scores,
            });
        }

        report.compute_summary();
        report
    }
}
