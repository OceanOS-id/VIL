// ── N02: Synthetic Generator ────────────────────────────────────────
use crate::quality::QualityChecker;
use crate::template::GenerationTemplate;
use serde::{Deserialize, Serialize};

/// A seed example for synthetic expansion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedExample {
    pub instruction: String,
    pub input: String,
    pub output: String,
}

/// A generated synthetic example with quality metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntheticExample {
    pub instruction: String,
    pub input: String,
    pub output: String,
    pub quality_score: f64,
    pub template_name: String,
}

/// Synthetic data generator — expands seed examples through templates.
#[derive(Debug, Clone)]
pub struct SyntheticGenerator {
    pub templates: Vec<GenerationTemplate>,
    pub quality_checker: QualityChecker,
}

impl SyntheticGenerator {
    pub fn new(templates: Vec<GenerationTemplate>, quality_checker: QualityChecker) -> Self {
        Self {
            templates,
            quality_checker,
        }
    }

    /// Generate a batch of synthetic examples from seeds.
    /// Cycles through templates and seeds to produce `count` examples.
    pub fn generate_batch(
        &self,
        seed_examples: &[SeedExample],
        count: usize,
    ) -> Vec<SyntheticExample> {
        if seed_examples.is_empty() || self.templates.is_empty() {
            return Vec::new();
        }

        let seed_texts: Vec<String> = seed_examples
            .iter()
            .map(|s| format!("{} {} {}", s.instruction, s.input, s.output))
            .collect();

        let mut results = Vec::with_capacity(count);

        for i in 0..count {
            let seed = &seed_examples[i % seed_examples.len()];
            let template = &self.templates[i % self.templates.len()];

            let rendered = template.render(&seed.instruction, &seed.input, &seed.output);

            let combined = format!(
                "{} {} {}",
                rendered.instruction, rendered.input, rendered.output
            );
            let quality_score = self.quality_checker.score(&combined, &seed_texts);

            results.push(SyntheticExample {
                instruction: rendered.instruction,
                input: rendered.input,
                output: rendered.output,
                quality_score,
                template_name: template.name.clone(),
            });
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::template::{instruction_template, qa_template};

    fn sample_seeds() -> Vec<SeedExample> {
        vec![
            SeedExample {
                instruction: "Explain photosynthesis".into(),
                input: "".into(),
                output: "Photosynthesis is the process by which plants convert sunlight.".into(),
            },
            SeedExample {
                instruction: "What is gravity?".into(),
                input: "".into(),
                output: "Gravity is a fundamental force of attraction.".into(),
            },
        ]
    }

    #[test]
    fn generate_batch_correct_count() {
        let gen = SyntheticGenerator::new(
            vec![qa_template(), instruction_template()],
            QualityChecker::default(),
        );
        let batch = gen.generate_batch(&sample_seeds(), 5);
        assert_eq!(batch.len(), 5);
    }

    #[test]
    fn generate_batch_has_quality_scores() {
        let gen = SyntheticGenerator::new(vec![qa_template()], QualityChecker::default());
        let batch = gen.generate_batch(&sample_seeds(), 3);
        for ex in &batch {
            assert!(ex.quality_score >= 0.0 && ex.quality_score <= 1.0);
        }
    }

    #[test]
    fn generate_batch_empty_seeds() {
        let gen = SyntheticGenerator::new(vec![qa_template()], QualityChecker::default());
        let batch = gen.generate_batch(&[], 5);
        assert!(batch.is_empty());
    }

    #[test]
    fn generate_batch_template_names() {
        let gen = SyntheticGenerator::new(
            vec![qa_template(), instruction_template()],
            QualityChecker::default(),
        );
        let batch = gen.generate_batch(&sample_seeds(), 4);
        assert_eq!(batch[0].template_name, "qa");
        assert_eq!(batch[1].template_name, "instruction");
    }
}
