use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::field::FieldType;

/// A single extracted field value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedField {
    pub name: String,
    pub value: String,
    pub field_type: FieldType,
    pub confidence: f32,
}

/// Result of an extraction run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    /// Map of field name -> extracted value.
    pub fields: HashMap<String, ExtractedField>,
    /// Overall confidence (average of individual field confidences).
    pub confidence: f32,
    /// Fields that were required but not found.
    pub missing_required: Vec<String>,
}

impl ExtractionResult {
    pub fn is_complete(&self) -> bool {
        self.missing_required.is_empty()
    }
}
