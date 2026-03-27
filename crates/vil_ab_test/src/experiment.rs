use serde::{Deserialize, Serialize};

use crate::variant::Variant;

/// Status of an experiment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExpStatus {
    Draft,
    Running,
    Paused,
    Completed,
}

/// An A/B test experiment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experiment {
    pub name: String,
    pub variants: Vec<Variant>,
    pub status: ExpStatus,
}

impl Experiment {
    pub fn new(name: impl Into<String>, variants: Vec<Variant>) -> Self {
        Self {
            name: name.into(),
            variants,
            status: ExpStatus::Draft,
        }
    }

    /// Assign a user to a variant using weighted random selection.
    /// Returns the variant name.
    pub fn assign(&self) -> &str {
        if self.variants.is_empty() {
            return "";
        }

        let total_weight: f32 = self.variants.iter().map(|v| v.weight).sum();
        if total_weight == 0.0 {
            return &self.variants[0].name;
        }

        // Simple deterministic-ish random using parking_lot + timestamp
        let rand_val = pseudo_random() * total_weight;
        let mut cumulative = 0.0_f32;

        for variant in &self.variants {
            cumulative += variant.weight;
            if rand_val < cumulative {
                return &variant.name;
            }
        }

        // Fallback to last variant
        &self.variants.last().unwrap().name
    }

    /// Record a conversion for the named variant.
    pub fn record_impression(&mut self, variant_name: &str) {
        if let Some(v) = self.variants.iter_mut().find(|v| v.name == variant_name) {
            v.impressions += 1;
        }
    }

    /// Record a conversion for the named variant.
    pub fn record_conversion(&mut self, variant_name: &str) {
        if let Some(v) = self.variants.iter_mut().find(|v| v.name == variant_name) {
            v.conversions += 1;
        }
    }

    pub fn start(&mut self) {
        self.status = ExpStatus::Running;
    }

    pub fn stop(&mut self) {
        self.status = ExpStatus::Completed;
    }
}

/// Simple pseudo-random float in [0, 1) using timestamp nanos.
fn pseudo_random() -> f32 {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    // Mix bits
    let mixed = nanos.wrapping_mul(2654435761);
    (mixed as f32) / (u32::MAX as f32)
}
