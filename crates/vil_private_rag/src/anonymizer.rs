//! Entity anonymization — replace real names with consistent pseudonyms.

use std::collections::HashMap;

/// Anonymizer that maps real entities to consistent pseudonyms.
pub struct Anonymizer {
    /// Mapping from real entity to pseudonym.
    mappings: HashMap<String, String>,
    /// Counter for generating new pseudonyms.
    counter: u64,
    /// Prefix for pseudonyms.
    prefix: String,
}

impl Anonymizer {
    pub fn new(prefix: &str) -> Self {
        Self {
            mappings: HashMap::new(),
            counter: 0,
            prefix: prefix.to_string(),
        }
    }

    /// Anonymize an entity name. Same input always produces same output.
    pub fn anonymize(&mut self, entity: &str) -> String {
        if let Some(pseudonym) = self.mappings.get(entity) {
            return pseudonym.clone();
        }

        self.counter += 1;
        // Hash-based pseudonym for consistency.
        let hash = simple_hash(entity);
        let pseudonym = format!("{}_{:04X}_{}", self.prefix, hash, self.counter);
        self.mappings.insert(entity.to_string(), pseudonym.clone());
        pseudonym
    }

    /// Anonymize all known entities in a text.
    pub fn anonymize_text(&mut self, text: &str, entities: &[String]) -> String {
        let mut result = text.to_string();
        // Sort entities by length (longest first) to avoid partial replacements.
        let mut sorted: Vec<&String> = entities.iter().collect();
        sorted.sort_by(|a, b| b.len().cmp(&a.len()));

        for entity in sorted {
            let pseudonym = self.anonymize(entity);
            result = result.replace(entity.as_str(), &pseudonym);
        }
        result
    }

    /// Get the current mapping table.
    pub fn mappings(&self) -> &HashMap<String, String> {
        &self.mappings
    }
}

impl Default for Anonymizer {
    fn default() -> Self {
        Self::new("PERSON")
    }
}

/// Simple deterministic hash for pseudonym generation.
fn simple_hash(s: &str) -> u16 {
    let mut h: u32 = 5381;
    for b in s.bytes() {
        h = h.wrapping_mul(33).wrapping_add(b as u32);
    }
    (h & 0xFFFF) as u16
}
