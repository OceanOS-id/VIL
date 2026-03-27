use serde::{Deserialize, Serialize};
use vil_macros::VilAiEvent;

use crate::experiment::Experiment;
use crate::stats;

/// Summary of a single variant's performance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantSummary {
    pub name: String,
    pub impressions: u64,
    pub conversions: u64,
    pub conversion_rate: f64,
}

/// Full experiment report with statistical analysis.
#[derive(Debug, Clone, Serialize, Deserialize, VilAiEvent)]
pub struct ExperimentReport {
    pub experiment_name: String,
    pub variants_summary: Vec<VariantSummary>,
    pub winner: Option<String>,
    pub significant: bool,
}

impl ExperimentReport {
    /// Generate a report for the given experiment.
    ///
    /// Compares each non-control variant against the first variant (control).
    pub fn generate(experiment: &Experiment) -> Self {
        let variants_summary: Vec<VariantSummary> = experiment
            .variants
            .iter()
            .map(|v| VariantSummary {
                name: v.name.clone(),
                impressions: v.impressions,
                conversions: v.conversions,
                conversion_rate: v.conversion_rate(),
            })
            .collect();

        if experiment.variants.len() < 2 {
            return Self {
                experiment_name: experiment.name.clone(),
                variants_summary,
                winner: None,
                significant: false,
            };
        }

        let control = &experiment.variants[0];
        let mut best_z = 0.0_f64;
        let mut winner: Option<String> = None;
        let mut any_significant = false;

        for variant in &experiment.variants[1..] {
            let result = stats::z_test(control, variant);
            if result.significant && result.z_score.abs() > best_z.abs() {
                best_z = result.z_score;
                any_significant = true;
                // Pick the variant with better conversion rate
                if variant.conversion_rate() > control.conversion_rate() {
                    winner = Some(variant.name.clone());
                } else {
                    winner = Some(control.name.clone());
                }
            }
        }

        Self {
            experiment_name: experiment.name.clone(),
            variants_summary,
            winner,
            significant: any_significant,
        }
    }
}
