//! VIL Evaluation Framework (H10).
//!
//! Provides metrics, datasets, batch evaluation, and reporting for LLM output quality.
//!
//! ```
//! use vil_eval::{EvalDataset, EvalCase, EvalRunner, AnswerRelevance};
//!
//! let mut dataset = EvalDataset::new();
//! dataset.add_case(EvalCase {
//!     question: "What is Rust?".to_string(),
//!     context: "Rust is a systems programming language.".to_string(),
//!     answer: "Rust is a systems programming language focused on safety.".to_string(),
//!     reference: None,
//! });
//! let runner = EvalRunner::new(dataset).add_metric(Box::new(AnswerRelevance));
//! let report = runner.run();
//! assert_eq!(report.case_count(), 1);
//! ```

pub mod dataset;
pub mod evaluator;
pub mod metrics;
pub mod report;
pub mod runner;

pub use dataset::{EvalCase, EvalDataset};
pub use evaluator::{EvalMetric, MetricScore};
pub use metrics::{AnswerLength, AnswerRelevance, ContextRecall, Faithfulness};
pub use report::{CaseResult, EvalReport};
pub use runner::EvalRunner;

// VIL integration layer
pub mod handlers;
pub mod pipeline_sse;
pub mod plugin;
pub mod semantic;

pub use plugin::EvalPlugin;
pub use semantic::{EvalFault, EvalRunEvent, EvalState};

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_dataset() -> EvalDataset {
        let mut ds = EvalDataset::new();
        ds.add_case(EvalCase {
            question: "What is Rust programming language?".to_string(),
            context: "Rust is a systems programming language focused on safety and performance."
                .to_string(),
            answer: "Rust is a systems programming language that prioritizes safety and speed."
                .to_string(),
            reference: Some(
                "Rust is a systems programming language focused on safety, speed, and concurrency."
                    .to_string(),
            ),
        });
        ds.add_case(EvalCase {
            question: "What is VIL?".to_string(),
            context: "VIL is a process-oriented server framework built in Rust.".to_string(),
            answer: "VIL is a framework for building servers using Rust.".to_string(),
            reference: Some(
                "VIL is a process-oriented server framework built in Rust.".to_string(),
            ),
        });
        ds
    }

    #[test]
    fn test_answer_relevance_scoring() {
        let metric = AnswerRelevance;
        let score = metric.evaluate(
            "What is Rust?",
            "Rust is a systems programming language.",
            "",
            None,
        );
        assert_eq!(score.name, "answer_relevance");
        assert!(score.score >= 0.0 && score.score <= 1.0);
        assert!(score.score > 0.0); // "Rust" and "is" overlap
    }

    #[test]
    fn test_faithfulness_scoring() {
        let metric = Faithfulness;
        let score = metric.evaluate(
            "",
            "Rust is a systems language.",
            "Rust is a systems programming language focused on safety.",
            None,
        );
        assert_eq!(score.name, "faithfulness");
        assert!(score.score >= 0.0 && score.score <= 1.0);
        assert!(score.score > 0.0);
    }

    #[test]
    fn test_context_recall_with_reference() {
        let metric = ContextRecall;
        let score = metric.evaluate(
            "",
            "",
            "Rust is safe and fast.",
            Some("Rust is safe, fast, and concurrent."),
        );
        assert_eq!(score.name, "context_recall");
        assert!(score.score > 0.0);
    }

    #[test]
    fn test_context_recall_no_reference() {
        let metric = ContextRecall;
        let score = metric.evaluate("", "", "Rust is safe.", None);
        assert_eq!(score.score, 0.0);
    }

    #[test]
    fn test_answer_length_too_short() {
        let metric = AnswerLength::default();
        let score = metric.evaluate("", "Yes.", "", None);
        assert!(score.score < 1.0);
    }

    #[test]
    fn test_answer_length_good() {
        let metric = AnswerLength::default();
        let score = metric.evaluate(
            "",
            "Rust is a systems programming language focused on safety and performance.",
            "",
            None,
        );
        assert_eq!(score.score, 1.0);
    }

    #[test]
    fn test_dataset_loading_from_json() {
        let json = r#"{
            "cases": [
                {
                    "question": "What is Rust?",
                    "context": "Rust is a language.",
                    "answer": "Rust is great.",
                    "reference": null
                }
            ]
        }"#;
        let ds = EvalDataset::from_json(json).unwrap();
        assert_eq!(ds.len(), 1);
        assert_eq!(ds.cases[0].question, "What is Rust?");
    }

    #[test]
    fn test_runner_with_multiple_metrics() {
        let ds = sample_dataset();
        let runner = EvalRunner::new(ds)
            .add_metric(Box::new(AnswerRelevance))
            .add_metric(Box::new(Faithfulness))
            .add_metric(Box::new(ContextRecall))
            .add_metric(Box::new(AnswerLength::default()));
        let report = runner.run();
        assert_eq!(report.case_count(), 2);
        assert_eq!(report.results[0].scores.len(), 4);
        assert!(report.summary.contains_key("answer_relevance"));
        assert!(report.summary.contains_key("faithfulness"));
        assert!(report.summary.contains_key("context_recall"));
        assert!(report.summary.contains_key("answer_length"));
    }

    #[test]
    fn test_empty_dataset() {
        let ds = EvalDataset::new();
        assert!(ds.is_empty());
        let runner = EvalRunner::new(ds).add_metric(Box::new(AnswerRelevance));
        let report = runner.run();
        assert_eq!(report.case_count(), 0);
        assert!(report.summary.is_empty());
    }

    #[test]
    fn test_report_aggregation() {
        let ds = sample_dataset();
        let runner = EvalRunner::new(ds)
            .add_metric(Box::new(AnswerRelevance))
            .add_metric(Box::new(Faithfulness));
        let report = runner.run();

        // Verify averages are computed correctly
        let relevance_avg = report.metric_average("answer_relevance").unwrap();
        let case_scores: Vec<f32> = report
            .results
            .iter()
            .map(|r| {
                r.scores
                    .iter()
                    .find(|s| s.name == "answer_relevance")
                    .unwrap()
                    .score
            })
            .collect();
        let expected_avg = case_scores.iter().sum::<f32>() / case_scores.len() as f32;
        assert!((relevance_avg - expected_avg).abs() < f32::EPSILON);
    }

    #[test]
    fn test_metric_scores_in_range() {
        let ds = sample_dataset();
        let runner = EvalRunner::new(ds)
            .add_metric(Box::new(AnswerRelevance))
            .add_metric(Box::new(Faithfulness))
            .add_metric(Box::new(ContextRecall))
            .add_metric(Box::new(AnswerLength::default()));
        let report = runner.run();

        for case_result in &report.results {
            for score in &case_result.scores {
                assert!(
                    score.score >= 0.0 && score.score <= 1.0,
                    "Metric {} score {} out of [0,1] range",
                    score.name,
                    score.score
                );
            }
        }
    }

    #[test]
    fn test_dataset_roundtrip_json() {
        let ds = sample_dataset();
        let json = ds.to_json().unwrap();
        let ds2 = EvalDataset::from_json(&json).unwrap();
        assert_eq!(ds2.len(), ds.len());
        assert_eq!(ds2.cases[0].question, ds.cases[0].question);
    }
}
