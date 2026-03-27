use serde::{Deserialize, Serialize};

/// A node in the memory graph representing a discrete piece of knowledge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: u64,
    pub name: String,
    pub entity_type: EntityType,
    pub attributes: serde_json::Value,
    pub created_at: u64,
    pub last_accessed: u64,
    pub access_count: u32,
    pub importance: f32,
}

/// Classification of an entity node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EntityType {
    Person,
    Concept,
    Event,
    Location,
    Fact,
    /// User preference remembered across sessions.
    Preference,
    /// Summary of a past conversation.
    Conversation,
    /// Application-defined type.
    Custom(String),
}
