// ── N01: DataPipeline ────────────────────────────────────────────────
use crate::dedup;
use crate::filter::QualityFilter;
use crate::formatter::{self, OutputFormat, TrainingRecord};
use serde::{Deserialize, Serialize};

/// A single step in the data-preparation pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineStep {
    Dedup,
    FuzzyDedup { threshold: f64 },
    Filter(QualityFilter),
    Format(OutputFormat),
    Custom(String),
}

/// Chainable data-preparation pipeline: load -> dedup -> filter -> format -> save.
#[derive(Debug, Clone)]
pub struct DataPipeline {
    pub steps: Vec<PipelineStep>,
}

/// Result of running the pipeline.
#[derive(Debug, Clone)]
pub struct PipelineResult {
    /// Texts surviving dedup + filter steps.
    pub texts: Vec<String>,
    /// If a Format step was present, the formatted output.
    pub formatted: Option<String>,
    /// Per-step counts for observability.
    pub step_counts: Vec<(String, usize)>,
}

impl DataPipeline {
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    pub fn add_step(mut self, step: PipelineStep) -> Self {
        self.steps.push(step);
        self
    }

    /// Run the pipeline over raw texts and (optionally) training records for formatting.
    pub fn run(&self, texts: &[String], records: Option<&[TrainingRecord]>) -> PipelineResult {
        let mut current = texts.to_vec();
        let mut formatted: Option<String> = None;
        let mut step_counts = Vec::new();

        for step in &self.steps {
            match step {
                PipelineStep::Dedup => {
                    current = dedup::exact_dedup(&current);
                    step_counts.push(("dedup".into(), current.len()));
                }
                PipelineStep::FuzzyDedup { threshold } => {
                    current = dedup::fuzzy_dedup(&current, *threshold);
                    step_counts.push(("fuzzy_dedup".into(), current.len()));
                }
                PipelineStep::Filter(qf) => {
                    current = qf.filter_batch(&current);
                    step_counts.push(("filter".into(), current.len()));
                }
                PipelineStep::Format(fmt) => {
                    if let Some(recs) = records {
                        formatted = Some(formatter::format_records(recs, *fmt));
                    }
                    step_counts.push(("format".into(), current.len()));
                }
                PipelineStep::Custom(name) => {
                    // Custom steps are no-ops in the base crate; extension point.
                    step_counts.push((format!("custom:{}", name), current.len()));
                }
            }
        }

        PipelineResult {
            texts: current,
            formatted,
            step_counts,
        }
    }
}

impl Default for DataPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_dedup_then_filter() {
        let pipe = DataPipeline::new()
            .add_step(PipelineStep::Dedup)
            .add_step(PipelineStep::Filter(QualityFilter::new(5, 1000, 0.0)));

        let input = vec![
            "Hello world foo bar".into(),
            "Hi".into(),
            "Hello world foo bar".into(),
            "Another good sentence here".into(),
        ];
        let result = pipe.run(&input, None);
        // After dedup: 3 texts. After filter (min_length=5): "Hi" removed -> 2.
        assert_eq!(result.texts.len(), 2);
        assert_eq!(result.step_counts.len(), 2);
    }

    #[test]
    fn pipeline_with_format() {
        let pipe = DataPipeline::new().add_step(PipelineStep::Format(OutputFormat::Jsonl));

        let records = vec![TrainingRecord::new("Do X", "", "Done X")];
        let result = pipe.run(&[], Some(&records));
        assert!(result.formatted.is_some());
        assert!(result.formatted.unwrap().contains("Do X"));
    }

    #[test]
    fn empty_pipeline() {
        let pipe = DataPipeline::new();
        let result = pipe.run(&["a".into(), "b".into()], None);
        assert_eq!(result.texts.len(), 2);
        assert!(result.step_counts.is_empty());
    }
}
