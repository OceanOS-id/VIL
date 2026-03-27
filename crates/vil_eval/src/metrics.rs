//! Built-in evaluation metrics.

use std::collections::HashSet;

use crate::evaluator::{EvalMetric, MetricScore};

/// Compute word-level overlap ratio between two texts.
fn word_overlap(a: &str, b: &str) -> f32 {
    let lower_a = a.to_lowercase();
    let lower_b = b.to_lowercase();
    let words_a: HashSet<&str> = lower_a.split_whitespace().collect();
    let words_b: HashSet<&str> = lower_b.split_whitespace().collect();

    if words_a.is_empty() || words_b.is_empty() {
        return 0.0;
    }

    let intersection = words_a.intersection(&words_b).count() as f32;
    let union = words_a.union(&words_b).count() as f32;
    intersection / union
}

/// Answer Relevance — keyword overlap between question and answer.
pub struct AnswerRelevance;

impl EvalMetric for AnswerRelevance {
    fn evaluate(
        &self,
        question: &str,
        answer: &str,
        _context: &str,
        _reference: Option<&str>,
    ) -> MetricScore {
        let score = word_overlap(question, answer).clamp(0.0, 1.0);
        MetricScore {
            name: "answer_relevance".to_string(),
            score,
            details: serde_json::json!({
                "method": "word_overlap",
                "question_words": question.split_whitespace().count(),
                "answer_words": answer.split_whitespace().count(),
            }),
        }
    }
}

/// Faithfulness — overlap between answer and context (is the answer grounded?).
pub struct Faithfulness;

impl EvalMetric for Faithfulness {
    fn evaluate(
        &self,
        _question: &str,
        answer: &str,
        context: &str,
        _reference: Option<&str>,
    ) -> MetricScore {
        let score = word_overlap(answer, context).clamp(0.0, 1.0);
        MetricScore {
            name: "faithfulness".to_string(),
            score,
            details: serde_json::json!({
                "method": "word_overlap_answer_context",
                "answer_words": answer.split_whitespace().count(),
                "context_words": context.split_whitespace().count(),
            }),
        }
    }
}

/// Context Recall — overlap between context and reference answer.
pub struct ContextRecall;

impl EvalMetric for ContextRecall {
    fn evaluate(
        &self,
        _question: &str,
        _answer: &str,
        context: &str,
        reference: Option<&str>,
    ) -> MetricScore {
        let score = match reference {
            Some(ref_text) => word_overlap(context, ref_text).clamp(0.0, 1.0),
            None => 0.0,
        };
        MetricScore {
            name: "context_recall".to_string(),
            score,
            details: serde_json::json!({
                "method": "word_overlap_context_reference",
                "has_reference": reference.is_some(),
            }),
        }
    }
}

/// Answer Length — penalizes too short or too long answers.
pub struct AnswerLength {
    /// Minimum ideal word count.
    pub min_words: usize,
    /// Maximum ideal word count.
    pub max_words: usize,
}

impl Default for AnswerLength {
    fn default() -> Self {
        Self {
            min_words: 5,
            max_words: 500,
        }
    }
}

impl EvalMetric for AnswerLength {
    fn evaluate(
        &self,
        _question: &str,
        answer: &str,
        _context: &str,
        _reference: Option<&str>,
    ) -> MetricScore {
        let word_count = answer.split_whitespace().count();
        let score = if word_count < self.min_words {
            word_count as f32 / self.min_words as f32
        } else if word_count > self.max_words {
            (self.max_words as f32 / word_count as f32).clamp(0.0, 1.0)
        } else {
            1.0
        };

        MetricScore {
            name: "answer_length".to_string(),
            score: score.clamp(0.0, 1.0),
            details: serde_json::json!({
                "word_count": word_count,
                "min_words": self.min_words,
                "max_words": self.max_words,
            }),
        }
    }
}
