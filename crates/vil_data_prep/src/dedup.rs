// ── N01: Deduplication ────────────────────────────────────────────────
use std::collections::HashSet;

/// Hash-based exact deduplication — O(n) with HashSet.
pub fn exact_dedup(texts: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for text in texts {
        if seen.insert(text.clone()) {
            result.push(text.clone());
        }
    }
    result
}

/// Jaccard-similarity fuzzy deduplication.
/// Keeps the first occurrence; drops later texts whose Jaccard similarity
/// to any already-kept text exceeds `threshold` (0.0–1.0).
pub fn fuzzy_dedup(texts: &[String], threshold: f64) -> Vec<String> {
    let shingled: Vec<HashSet<&str>> = texts.iter().map(|t| shingle(t, 3)).collect();
    let mut kept: Vec<usize> = Vec::new();

    for (i, shingles_i) in shingled.iter().enumerate() {
        let is_dup = kept
            .iter()
            .any(|&j| jaccard(shingles_i, &shingled[j]) >= threshold);
        if !is_dup {
            kept.push(i);
        }
    }

    kept.into_iter().map(|i| texts[i].clone()).collect()
}

fn shingle(text: &str, n: usize) -> HashSet<&str> {
    let mut set = HashSet::new();
    if text.len() >= n {
        for i in 0..=text.len() - n {
            if text.is_char_boundary(i) && text.is_char_boundary(i + n) {
                set.insert(&text[i..i + n]);
            }
        }
    }
    set
}

fn jaccard<'a>(a: &HashSet<&'a str>, b: &HashSet<&'a str>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let intersection = a.intersection(b).count() as f64;
    let union = a.union(b).count() as f64;
    if union == 0.0 {
        1.0
    } else {
        intersection / union
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_dedup_removes_duplicates() {
        let input = vec![
            "hello world".into(),
            "foo bar".into(),
            "hello world".into(),
            "baz".into(),
            "foo bar".into(),
        ];
        let result = exact_dedup(&input);
        assert_eq!(result, vec!["hello world", "foo bar", "baz"]);
    }

    #[test]
    fn exact_dedup_empty() {
        let result = exact_dedup(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn fuzzy_dedup_removes_similar() {
        let input = vec![
            "the quick brown fox".into(),
            "the quick brown fox jumps".into(), // very similar
            "completely different text here".into(),
        ];
        let result = fuzzy_dedup(&input, 0.6);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "the quick brown fox");
        assert_eq!(result[1], "completely different text here");
    }

    #[test]
    fn fuzzy_dedup_keeps_all_when_threshold_high() {
        let input = vec!["alpha beta gamma".into(), "alpha beta delta".into()];
        let result = fuzzy_dedup(&input, 0.99);
        assert_eq!(result.len(), 2);
    }
}
