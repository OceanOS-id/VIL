use serde::{Deserialize, Serialize};

/// A variant in an A/B test experiment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variant {
    /// Variant name (e.g., "control", "treatment_a").
    pub name: String,
    /// Traffic weight (0.0–1.0). All variant weights in an experiment should sum to ~1.0.
    pub weight: f32,
    /// Number of times this variant was shown.
    pub impressions: u64,
    /// Number of successful conversions.
    pub conversions: u64,
}

impl Variant {
    pub fn new(name: impl Into<String>, weight: f32) -> Self {
        Self {
            name: name.into(),
            weight,
            impressions: 0,
            conversions: 0,
        }
    }

    /// Conversion rate as a proportion (0.0–1.0).
    pub fn conversion_rate(&self) -> f64 {
        if self.impressions == 0 {
            0.0
        } else {
            self.conversions as f64 / self.impressions as f64
        }
    }
}
