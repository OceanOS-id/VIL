use serde::{Deserialize, Serialize};

use crate::variant::Variant;

/// Result of a statistical significance test.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignificanceResult {
    pub z_score: f64,
    pub p_value: f64,
    pub significant: bool,
    pub confidence_level: f32,
}

/// Perform a two-proportion z-test between control and treatment variants.
///
/// Tests whether the treatment conversion rate is significantly different
/// from the control conversion rate.
pub fn z_test(control: &Variant, treatment: &Variant) -> SignificanceResult {
    let n1 = control.impressions as f64;
    let n2 = treatment.impressions as f64;

    if n1 == 0.0 || n2 == 0.0 {
        return SignificanceResult {
            z_score: 0.0,
            p_value: 1.0,
            significant: false,
            confidence_level: 0.0,
        };
    }

    let p1 = control.conversion_rate();
    let p2 = treatment.conversion_rate();

    // Pooled proportion
    let p_pool = (control.conversions as f64 + treatment.conversions as f64) / (n1 + n2);

    let se = (p_pool * (1.0 - p_pool) * (1.0 / n1 + 1.0 / n2)).sqrt();

    if se == 0.0 {
        return SignificanceResult {
            z_score: 0.0,
            p_value: 1.0,
            significant: false,
            confidence_level: 0.0,
        };
    }

    let z = (p2 - p1) / se;
    let p_value = two_tail_p_value(z);

    let (significant, confidence_level) = if p_value < 0.01 {
        (true, 0.99)
    } else if p_value < 0.05 {
        (true, 0.95)
    } else if p_value < 0.10 {
        (false, 0.90)
    } else {
        (false, 0.0)
    };

    SignificanceResult {
        z_score: z,
        p_value,
        significant,
        confidence_level,
    }
}

/// Approximate two-tailed p-value from z-score using the complementary
/// error function approximation.
fn two_tail_p_value(z: f64) -> f64 {
    // Abramowitz & Stegun approximation of the normal CDF
    let x = z.abs();
    let t = 1.0 / (1.0 + 0.2316419 * x);
    let d = 0.3989422804014327; // 1/sqrt(2*pi)
    let p = d * (-x * x / 2.0).exp();
    let poly = t * (0.319381530
        + t * (-0.356563782
            + t * (1.781477937 + t * (-1.821255978 + t * 1.330274429))));
    let one_tail = p * poly;
    (2.0 * one_tail).min(1.0)
}
