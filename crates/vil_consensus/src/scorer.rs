use std::collections::HashSet;

/// Quality score breakdown for a single response.
#[derive(Debug, Clone)]
pub struct ResponseScore {
    pub total: f32,
    pub length: f32,
    pub structure: f32,
    pub coherence: f32,
}

/// Score a response for quality heuristics.
pub fn score_response(response: &str) -> ResponseScore {
    let length_score = (response.len() as f32 / 500.0).min(1.0);

    let has_structure =
        response.contains('\n') || response.contains("1.") || response.contains("- ");
    let structure_score = if has_structure { 0.3 } else { 0.0 };

    let coherence_score = 1.0 - (response.matches("I don't know").count() as f32 * 0.3).min(1.0);

    ResponseScore {
        total: (length_score * 0.3 + structure_score + coherence_score * 0.4).min(1.0),
        length: length_score,
        structure: structure_score,
        coherence: coherence_score,
    }
}

/// Compute text similarity between two strings using Jaccard index on word sets.
pub fn text_similarity(a: &str, b: &str) -> f32 {
    let words_a: HashSet<&str> = a.split_whitespace().collect();
    let words_b: HashSet<&str> = b.split_whitespace().collect();

    if words_a.is_empty() && words_b.is_empty() {
        return 1.0;
    }

    let intersection = words_a.intersection(&words_b).count() as f32;
    let union = words_a.union(&words_b).count() as f32;

    if union == 0.0 {
        return 0.0;
    }

    intersection / union
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_short_response() {
        let score = score_response("ok");
        assert!(score.total > 0.0);
        assert!(score.length < 0.1);
    }

    #[test]
    fn test_score_structured_response() {
        let response = "Here is the answer:\n1. First point\n2. Second point\n- Detail";
        let score = score_response(response);
        assert!(score.structure > 0.0);
    }

    #[test]
    fn test_score_incoherent_response() {
        let response = "I don't know I don't know I don't know";
        let score = score_response(response);
        assert!(score.coherence < 0.5);
    }

    #[test]
    fn test_similarity_identical() {
        let sim = text_similarity("hello world foo bar", "hello world foo bar");
        assert!((sim - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_similarity_completely_different() {
        let sim = text_similarity("apple banana cherry", "dog elephant fox");
        assert!(sim < f32::EPSILON);
    }

    #[test]
    fn test_similarity_partial_overlap() {
        let sim = text_similarity("the quick brown fox", "the slow brown dog");
        // overlap: "the", "brown" = 2, union: "the","quick","brown","fox","slow","dog" = 6
        assert!((sim - 2.0 / 6.0).abs() < 0.01);
    }

    #[test]
    fn test_similarity_empty() {
        assert!((text_similarity("", "") - 1.0).abs() < f32::EPSILON);
    }
}
