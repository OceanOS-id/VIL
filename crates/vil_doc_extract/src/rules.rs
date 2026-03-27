use regex::Regex;
use std::collections::HashMap;

use crate::extractor::DataExtractor;
use crate::field::{FieldDef, FieldType};
use crate::result::{ExtractedField, ExtractionResult};

/// Rule-based (regex) field extractor.
#[derive(Debug, Default)]
pub struct RuleExtractor;

impl RuleExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl DataExtractor for RuleExtractor {
    fn extract(&self, text: &str, fields: &[FieldDef]) -> ExtractionResult {
        let mut extracted = HashMap::new();
        let mut missing_required = Vec::new();

        for field in fields {
            let mut found = false;
            for pattern in &field.patterns {
                if let Ok(re) = Regex::new(pattern) {
                    if let Some(caps) = re.captures(text) {
                        // Use capture group 1 if it exists, otherwise group 0
                        let value = caps
                            .get(1)
                            .or_else(|| caps.get(0))
                            .map(|m| m.as_str().to_string())
                            .unwrap_or_default();

                        let confidence = compute_confidence(&field.field_type, &value);

                        extracted.insert(
                            field.name.clone(),
                            ExtractedField {
                                name: field.name.clone(),
                                value,
                                field_type: field.field_type.clone(),
                                confidence,
                            },
                        );
                        found = true;
                        break;
                    }
                }
            }
            if !found && field.required {
                missing_required.push(field.name.clone());
            }
        }

        let confidence = if extracted.is_empty() {
            0.0
        } else {
            let sum: f32 = extracted.values().map(|f| f.confidence).sum();
            sum / extracted.len() as f32
        };

        ExtractionResult {
            fields: extracted,
            confidence,
            missing_required,
        }
    }
}

fn compute_confidence(field_type: &FieldType, value: &str) -> f32 {
    if value.is_empty() {
        return 0.0;
    }
    match field_type {
        FieldType::Email => {
            if value.contains('@') && value.contains('.') {
                0.95
            } else {
                0.3
            }
        }
        FieldType::Phone => {
            let digits: usize = value.chars().filter(|c| c.is_ascii_digit()).count();
            if digits >= 10 {
                0.9
            } else if digits >= 7 {
                0.7
            } else {
                0.4
            }
        }
        FieldType::Currency => {
            if value.chars().any(|c| c.is_ascii_digit()) {
                0.9
            } else {
                0.3
            }
        }
        FieldType::Date => 0.85,
        FieldType::Number => {
            if value.parse::<f64>().is_ok() {
                0.95
            } else {
                0.5
            }
        }
        FieldType::Text | FieldType::Address => 0.8,
    }
}

// ---------------------------------------------------------------------------
// Pre-built templates
// ---------------------------------------------------------------------------

/// Pre-built field definitions for invoice extraction.
pub fn invoice_fields() -> Vec<FieldDef> {
    vec![
        FieldDef::new("invoice_number", FieldType::Text, vec![
            r"(?i)invoice\s*#?\s*:?\s*([A-Z0-9\-]+)".into(),
            r"(?i)inv\s*#?\s*:?\s*([A-Z0-9\-]+)".into(),
        ], true),
        FieldDef::new("date", FieldType::Date, vec![
            r"(\d{4}-\d{2}-\d{2})".into(),
            r"(\d{2}/\d{2}/\d{4})".into(),
            r"(\d{2}-\d{2}-\d{4})".into(),
        ], true),
        FieldDef::new("total", FieldType::Currency, vec![
            r"(?i)total\s*:?\s*\$?([\d,]+\.?\d*)".into(),
            r"\$([\d,]+\.\d{2})".into(),
        ], true),
        FieldDef::new("email", FieldType::Email, vec![
            r"([a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,})".into(),
        ], false),
    ]
}

/// Pre-built field definitions for receipt extraction.
pub fn receipt_fields() -> Vec<FieldDef> {
    vec![
        FieldDef::new("store_name", FieldType::Text, vec![
            r"^(.+)$".into(),
        ], false),
        FieldDef::new("date", FieldType::Date, vec![
            r"(\d{4}-\d{2}-\d{2})".into(),
            r"(\d{2}/\d{2}/\d{4})".into(),
        ], false),
        FieldDef::new("total", FieldType::Currency, vec![
            r"(?i)total\s*:?\s*\$?([\d,]+\.?\d*)".into(),
        ], true),
        FieldDef::new("tax", FieldType::Currency, vec![
            r"(?i)tax\s*:?\s*\$?([\d,]+\.?\d*)".into(),
        ], false),
    ]
}

/// Pre-built field definitions for resume extraction.
pub fn resume_fields() -> Vec<FieldDef> {
    vec![
        FieldDef::new("email", FieldType::Email, vec![
            r"([a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,})".into(),
        ], true),
        FieldDef::new("phone", FieldType::Phone, vec![
            r"(\+?[\d\-\(\)\s]{10,})".into(),
            r"(\d{3}[\-\.]\d{3}[\-\.]\d{4})".into(),
        ], false),
        FieldDef::new("name", FieldType::Text, vec![
            r"^([A-Z][a-z]+\s+[A-Z][a-z]+)".into(),
        ], false),
    ]
}
