use crate::field::FieldDef;
use crate::result::ExtractionResult;

/// Trait for data extraction strategies.
pub trait DataExtractor: Send + Sync {
    /// Extract fields from text according to field definitions.
    fn extract(&self, text: &str, fields: &[FieldDef]) -> ExtractionResult;
}
