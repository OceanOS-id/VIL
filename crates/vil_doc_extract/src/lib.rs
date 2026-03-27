//! # vil_doc_extract (I06)
//!
//! Data Extraction Pipeline — rule-based field extraction from unstructured text.
//!
//! Provides a `DataExtractor` trait with a `RuleExtractor` implementation that
//! uses regex patterns to pull structured fields from raw text. Includes pre-built
//! templates for invoices, receipts, and resumes.

pub mod extractor;
pub mod field;
pub mod result;
pub mod rules;
pub mod semantic;
pub mod handlers;
pub mod plugin;
pub mod pipeline_sse;

pub use extractor::DataExtractor;
pub use field::{FieldDef, FieldType};
pub use result::{ExtractedField, ExtractionResult};
pub use rules::{RuleExtractor, invoice_fields, receipt_fields, resume_fields};
pub use plugin::DocExtractPlugin;
pub use semantic::{ExtractEvent, ExtractFault, ExtractFaultType, ExtractState};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_extraction() {
        let ext = RuleExtractor::new();
        let fields = vec![FieldDef::new(
            "email", FieldType::Email,
            vec![r"([a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,})".into()],
            true,
        )];
        let result = ext.extract("Contact us at alice@example.com for details.", &fields);
        assert!(result.is_complete());
        assert_eq!(result.fields["email"].value, "alice@example.com");
        assert!(result.fields["email"].confidence > 0.9);
    }

    #[test]
    fn test_date_extraction() {
        let ext = RuleExtractor::new();
        let fields = vec![FieldDef::new(
            "date", FieldType::Date,
            vec![r"(\d{4}-\d{2}-\d{2})".into()],
            true,
        )];
        let result = ext.extract("Invoice date: 2025-03-15", &fields);
        assert!(result.is_complete());
        assert_eq!(result.fields["date"].value, "2025-03-15");
    }

    #[test]
    fn test_currency_extraction() {
        let ext = RuleExtractor::new();
        let fields = vec![FieldDef::new(
            "total", FieldType::Currency,
            vec![r"\$([\d,]+\.\d{2})".into()],
            true,
        )];
        let result = ext.extract("Grand Total: $1,234.56", &fields);
        assert!(result.is_complete());
        assert_eq!(result.fields["total"].value, "1,234.56");
    }

    #[test]
    fn test_phone_extraction() {
        let ext = RuleExtractor::new();
        let fields = vec![FieldDef::new(
            "phone", FieldType::Phone,
            vec![r"(\d{3}-\d{3}-\d{4})".into()],
            true,
        )];
        let result = ext.extract("Call 555-123-4567 now", &fields);
        assert!(result.is_complete());
        assert_eq!(result.fields["phone"].value, "555-123-4567");
    }

    #[test]
    fn test_invoice_template() {
        let ext = RuleExtractor::new();
        let text = "Invoice #INV-2025-001\nDate: 2025-03-15\nTotal: $500.00\nEmail: billing@acme.com";
        let result = ext.extract(text, &invoice_fields());
        assert!(result.is_complete());
        assert!(result.fields.contains_key("invoice_number"));
        assert!(result.fields.contains_key("date"));
        assert!(result.fields.contains_key("total"));
        assert!(result.fields.contains_key("email"));
    }

    #[test]
    fn test_receipt_template() {
        let ext = RuleExtractor::new();
        let text = "SuperMart\n2025-01-10\nSubtotal: $45.00\nTax: $3.60\nTotal: $48.60";
        let result = ext.extract(text, &receipt_fields());
        assert!(result.is_complete());
        assert!(result.fields.contains_key("total"));
    }

    #[test]
    fn test_resume_template() {
        let ext = RuleExtractor::new();
        let text = "John Doe\njohn.doe@example.com\n555-987-6543\nSoftware Engineer";
        let result = ext.extract(text, &resume_fields());
        assert!(result.is_complete());
        assert_eq!(result.fields["email"].value, "john.doe@example.com");
    }

    #[test]
    fn test_missing_required_field() {
        let ext = RuleExtractor::new();
        let fields = vec![
            FieldDef::new("email", FieldType::Email,
                vec![r"([a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,})".into()],
                true),
            FieldDef::new("phone", FieldType::Phone,
                vec![r"(\d{3}-\d{3}-\d{4})".into()],
                true),
        ];
        let result = ext.extract("No contact info here.", &fields);
        assert!(!result.is_complete());
        assert_eq!(result.missing_required.len(), 2);
        assert!(result.missing_required.contains(&"email".to_string()));
        assert!(result.missing_required.contains(&"phone".to_string()));
    }

    #[test]
    fn test_number_extraction() {
        let ext = RuleExtractor::new();
        let fields = vec![FieldDef::new(
            "quantity", FieldType::Number,
            vec![r"(?i)qty\s*:?\s*(\d+)".into()],
            true,
        )];
        let result = ext.extract("Item: Widget, Qty: 42", &fields);
        assert!(result.is_complete());
        assert_eq!(result.fields["quantity"].value, "42");
    }

    #[test]
    fn test_overall_confidence() {
        let ext = RuleExtractor::new();
        let fields = vec![
            FieldDef::new("email", FieldType::Email,
                vec![r"([a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,})".into()],
                false),
        ];
        let result = ext.extract("test@example.com", &fields);
        assert!(result.confidence > 0.0);
    }
}
