use serde::{Deserialize, Serialize};

/// A directed edge connecting two entities in the memory graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub from: u64,
    pub to: u64,
    pub relation_type: RelationType,
    pub weight: f32,
    pub created_at: u64,
    pub metadata: serde_json::Value,
}

/// Classification of a relation edge.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RelationType {
    RelatedTo,
    IsA,
    HasProperty,
    Causes,
    PartOf,
    PreferenceFor,
    MentionedIn,
    FollowedBy,
    Custom(String),
}
