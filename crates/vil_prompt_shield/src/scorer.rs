use crate::result::RiskLevel;

/// Calculate overall risk score from individual threat risks.
pub fn calculate_score(risks: &[RiskLevel]) -> f64 {
    if risks.is_empty() {
        return 0.0;
    }

    let sum: f64 = risks
        .iter()
        .map(|r| match r {
            RiskLevel::None => 0.0,
            RiskLevel::Low => 0.1,
            RiskLevel::Medium => 0.3,
            RiskLevel::High => 0.6,
            RiskLevel::Critical => 0.9,
        })
        .sum();

    // Normalized: multiple threats compound
    (sum / risks.len() as f64 * (1.0 + (risks.len() as f64 - 1.0) * 0.1)).min(1.0)
}

/// Determine overall risk level from score.
pub fn score_to_risk(score: f64) -> RiskLevel {
    if score >= 0.8 {
        RiskLevel::Critical
    } else if score >= 0.5 {
        RiskLevel::High
    } else if score >= 0.3 {
        RiskLevel::Medium
    } else if score >= 0.1 {
        RiskLevel::Low
    } else {
        RiskLevel::None
    }
}

/// Additional heuristic checks (not pattern-based).
pub fn heuristic_score(text: &str) -> f64 {
    let mut score: f64 = 0.0;

    // Excessive special characters (encoding attacks)
    let special_ratio = text
        .chars()
        .filter(|c| !c.is_alphanumeric() && !c.is_whitespace())
        .count() as f64
        / text.len().max(1) as f64;
    if special_ratio > 0.3 {
        score += 0.1;
    }

    // Very long input (potential stuffing attack)
    if text.len() > 10000 {
        score += 0.1;
    }

    // Multiple newlines (prompt structure manipulation)
    let newline_count = text.matches('\n').count();
    if newline_count > 20 {
        score += 0.05;
    }

    // Mixed languages/scripts (obfuscation)
    let has_latin = text.chars().any(|c| c.is_ascii_alphabetic());
    let has_cjk = text.chars().any(|c| ('\u{4E00}'..='\u{9FFF}').contains(&c));
    let has_cyrillic = text
        .chars()
        .any(|c| ('\u{0400}'..='\u{04FF}').contains(&c));
    let script_count = [has_latin, has_cjk, has_cyrillic]
        .iter()
        .filter(|&&x| x)
        .count();
    if script_count > 2 {
        score += 0.05;
    }

    score.min(0.5) // heuristics alone shouldn't exceed 0.5
}
