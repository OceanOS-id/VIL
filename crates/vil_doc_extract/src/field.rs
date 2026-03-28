use serde::{Deserialize, Serialize};

/// Supported field types for extraction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FieldType {
    Text,
    Number,
    Date,
    Currency,
    Email,
    Phone,
    Address,
}

/// Definition of a field to extract from text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDef {
    /// Field name / key in the result map.
    pub name: String,
    /// Expected type of the field value.
    pub field_type: FieldType,
    /// Regex patterns to try (first match wins).
    pub patterns: Vec<String>,
    /// Whether the field is required.
    pub required: bool,
}

impl FieldDef {
    pub fn new(
        name: impl Into<String>,
        field_type: FieldType,
        patterns: Vec<String>,
        required: bool,
    ) -> Self {
        Self {
            name: name.into(),
            field_type,
            patterns,
            required,
        }
    }
}
