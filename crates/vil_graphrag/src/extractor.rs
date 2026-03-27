use async_trait::async_trait;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// An entity extracted from text.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtractedEntity {
    /// The entity text as found in the source.
    pub text: String,
    /// Classification of the entity.
    pub entity_type: ExtractedEntityType,
    /// Character offset in the source text.
    pub offset: usize,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f32,
}

/// Types of entities that can be extracted.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExtractedEntityType {
    Person,
    Organization,
    Date,
    Location,
    Email,
    Url,
    Number,
    Custom(String),
}

/// Trait for extracting entities from text.
#[async_trait]
pub trait EntityExtractor: Send + Sync {
    /// Extract entities from the given text.
    async fn extract(&self, text: &str) -> Vec<ExtractedEntity>;

    /// Name of this extractor.
    fn name(&self) -> &str;
}

/// Regex-based keyword entity extractor (NER approximation).
pub struct KeywordEntityExtractor {
    email_re: Regex,
    url_re: Regex,
    date_re: Regex,
    capitalized_re: Regex,
}

impl KeywordEntityExtractor {
    pub fn new() -> Self {
        Self {
            email_re: Regex::new(r#"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b"#).unwrap(),
            url_re: Regex::new(r#"https?://[^\s<>"']+"#).unwrap(),
            date_re: Regex::new(r#"\b\d{4}-\d{2}-\d{2}\b"#).unwrap(),
            capitalized_re: Regex::new(r#"\b([A-Z][a-z]+(?:\s+[A-Z][a-z]+)+)\b"#).unwrap(),
        }
    }
}

impl Default for KeywordEntityExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EntityExtractor for KeywordEntityExtractor {
    async fn extract(&self, text: &str) -> Vec<ExtractedEntity> {
        let mut entities = Vec::new();

        // Extract emails
        for mat in self.email_re.find_iter(text) {
            entities.push(ExtractedEntity {
                text: mat.as_str().to_string(),
                entity_type: ExtractedEntityType::Email,
                offset: mat.start(),
                confidence: 0.95,
            });
        }

        // Extract URLs
        for mat in self.url_re.find_iter(text) {
            entities.push(ExtractedEntity {
                text: mat.as_str().to_string(),
                entity_type: ExtractedEntityType::Url,
                offset: mat.start(),
                confidence: 0.95,
            });
        }

        // Extract dates (YYYY-MM-DD)
        for mat in self.date_re.find_iter(text) {
            entities.push(ExtractedEntity {
                text: mat.as_str().to_string(),
                entity_type: ExtractedEntityType::Date,
                offset: mat.start(),
                confidence: 0.85,
            });
        }

        // Extract capitalized phrases (likely names/orgs)
        for mat in self.capitalized_re.find_iter(text) {
            let phrase = mat.as_str();
            // Heuristic: 2 words = Person, 3+ words = Organization
            let word_count = phrase.split_whitespace().count();
            let etype = if word_count <= 2 {
                ExtractedEntityType::Person
            } else {
                ExtractedEntityType::Organization
            };
            entities.push(ExtractedEntity {
                text: phrase.to_string(),
                entity_type: etype,
                offset: mat.start(),
                confidence: 0.6,
            });
        }

        entities
    }

    fn name(&self) -> &str {
        "keyword_entity_extractor"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_extract_email() {
        let ext = KeywordEntityExtractor::new();
        let entities = ext.extract("Contact alice@example.com for info").await;
        assert!(entities.iter().any(|e| e.entity_type == ExtractedEntityType::Email
            && e.text == "alice@example.com"));
    }

    #[tokio::test]
    async fn test_extract_url() {
        let ext = KeywordEntityExtractor::new();
        let entities = ext.extract("Visit https://example.com/page for details").await;
        assert!(entities.iter().any(|e| e.entity_type == ExtractedEntityType::Url));
    }

    #[tokio::test]
    async fn test_extract_date() {
        let ext = KeywordEntityExtractor::new();
        let entities = ext.extract("The event is on 2025-03-15 at noon").await;
        assert!(entities.iter().any(|e| e.entity_type == ExtractedEntityType::Date
            && e.text == "2025-03-15"));
    }

    #[tokio::test]
    async fn test_extract_names() {
        let ext = KeywordEntityExtractor::new();
        let entities = ext.extract("John Smith works at Acme Corporation Ltd").await;
        assert!(entities.iter().any(|e| e.entity_type == ExtractedEntityType::Person
            && e.text.contains("John Smith")));
    }

    #[tokio::test]
    async fn test_extract_empty_text() {
        let ext = KeywordEntityExtractor::new();
        let entities = ext.extract("").await;
        assert!(entities.is_empty());
    }

    #[tokio::test]
    async fn test_extract_no_entities() {
        let ext = KeywordEntityExtractor::new();
        let entities = ext.extract("this is all lowercase with no special patterns").await;
        assert!(entities.is_empty());
    }
}
