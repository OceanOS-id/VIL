//! # vil_ab_test (I08)
//!
//! A/B Test Framework — experiment management with statistical significance testing.
//!
//! Supports weighted variant assignment, impression/conversion tracking,
//! two-proportion z-test for significance, and experiment reporting.

pub mod experiment;
pub mod report;
pub mod stats;
pub mod variant;
pub mod semantic;
pub mod handlers;
pub mod plugin;
pub mod pipeline_sse;

pub use experiment::{ExpStatus, Experiment};
pub use report::ExperimentReport;
pub use stats::{z_test, SignificanceResult};
pub use variant::Variant;
pub use handlers::ExperimentRegistry;
pub use plugin::AbTestPlugin;
pub use semantic::{AbTestEvent, AbTestFault, AbTestFaultType, AbTestState};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weighted_assignment_returns_valid_variant() {
        let exp = Experiment::new("test", vec![
            Variant::new("control", 0.5),
            Variant::new("treatment", 0.5),
        ]);
        let assigned = exp.assign();
        assert!(assigned == "control" || assigned == "treatment");
    }

    #[test]
    fn test_single_variant_always_assigned() {
        let exp = Experiment::new("test", vec![Variant::new("only", 1.0)]);
        for _ in 0..100 {
            assert_eq!(exp.assign(), "only");
        }
    }

    #[test]
    fn test_conversion_recording() {
        let mut exp = Experiment::new("test", vec![
            Variant::new("control", 0.5),
            Variant::new("treatment", 0.5),
        ]);
        exp.record_impression("control");
        exp.record_impression("control");
        exp.record_conversion("control");
        assert_eq!(exp.variants[0].impressions, 2);
        assert_eq!(exp.variants[0].conversions, 1);
        assert!((exp.variants[0].conversion_rate() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_z_test_known_values() {
        // Control: 100/1000 = 10%, Treatment: 150/1000 = 15%
        let mut control = Variant::new("control", 0.5);
        control.impressions = 1000;
        control.conversions = 100;

        let mut treatment = Variant::new("treatment", 0.5);
        treatment.impressions = 1000;
        treatment.conversions = 150;

        let result = z_test(&control, &treatment);
        // z should be positive (treatment > control)
        assert!(result.z_score > 0.0);
        // With this sample size, should be significant
        assert!(result.significant);
        assert!(result.p_value < 0.05);
    }

    #[test]
    fn test_z_test_no_difference() {
        let mut control = Variant::new("control", 0.5);
        control.impressions = 1000;
        control.conversions = 100;

        let mut treatment = Variant::new("treatment", 0.5);
        treatment.impressions = 1000;
        treatment.conversions = 100;

        let result = z_test(&control, &treatment);
        assert!((result.z_score).abs() < 0.01);
        assert!(!result.significant);
    }

    #[test]
    fn test_significance_detection_large_effect() {
        let mut control = Variant::new("control", 0.5);
        control.impressions = 5000;
        control.conversions = 250; // 5%

        let mut treatment = Variant::new("treatment", 0.5);
        treatment.impressions = 5000;
        treatment.conversions = 500; // 10%

        let result = z_test(&control, &treatment);
        assert!(result.significant);
        assert!(result.confidence_level >= 0.95);
    }

    #[test]
    fn test_no_data_not_significant() {
        let control = Variant::new("control", 0.5);
        let treatment = Variant::new("treatment", 0.5);
        let result = z_test(&control, &treatment);
        assert!(!result.significant);
        assert_eq!(result.z_score, 0.0);
        assert_eq!(result.p_value, 1.0);
    }

    #[test]
    fn test_experiment_report_with_winner() {
        let mut exp = Experiment::new("pricing", vec![
            Variant::new("control", 0.5),
            Variant::new("treatment", 0.5),
        ]);
        // Simulate data
        exp.variants[0].impressions = 2000;
        exp.variants[0].conversions = 100; // 5%
        exp.variants[1].impressions = 2000;
        exp.variants[1].conversions = 200; // 10%

        let report = ExperimentReport::generate(&exp);
        assert!(report.significant);
        assert_eq!(report.winner, Some("treatment".to_string()));
        assert_eq!(report.variants_summary.len(), 2);
    }

    #[test]
    fn test_experiment_report_no_data() {
        let exp = Experiment::new("empty", vec![
            Variant::new("control", 0.5),
            Variant::new("treatment", 0.5),
        ]);
        let report = ExperimentReport::generate(&exp);
        assert!(!report.significant);
        assert!(report.winner.is_none());
    }

    #[test]
    fn test_experiment_status_transitions() {
        let mut exp = Experiment::new("test", vec![Variant::new("a", 1.0)]);
        assert_eq!(exp.status, ExpStatus::Draft);
        exp.start();
        assert_eq!(exp.status, ExpStatus::Running);
        exp.stop();
        assert_eq!(exp.status, ExpStatus::Completed);
    }

    #[test]
    fn test_variant_conversion_rate_zero_impressions() {
        let v = Variant::new("test", 1.0);
        assert_eq!(v.conversion_rate(), 0.0);
    }
}
