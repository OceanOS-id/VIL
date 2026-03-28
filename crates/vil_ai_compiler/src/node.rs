//! Pipeline node types for AI workflow DAGs.

use serde::{Deserialize, Serialize};

/// Represents a single node in an AI pipeline DAG.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PipelineNode {
    /// Embed text into vector space.
    Embed { model: String, dimensions: usize },
    /// Search a vector store or index.
    Search { index: String, top_k: usize },
    /// Rerank search results for relevance.
    Rerank { model: String, top_n: usize },
    /// Generate text via LLM.
    Generate {
        model: String,
        max_tokens: usize,
        temperature: f64,
    },
    /// Transform data (map, project, reshape).
    Transform { operation: String },
    /// Filter data based on a predicate expression.
    Filter { predicate: String },
    /// Branch execution based on a condition.
    Branch { condition: String },
    /// Merge multiple upstream branches.
    Merge { strategy: MergeStrategy },
    /// Cache intermediate results.
    Cache { ttl_secs: u64, key_expr: String },
}

/// Strategy used when merging multiple branches.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MergeStrategy {
    /// Concatenate all inputs.
    Concat,
    /// Pick the first available result.
    First,
    /// Collect all inputs into a list.
    Collect,
}

impl PipelineNode {
    /// Returns a short human-readable label for the node type.
    pub fn type_name(&self) -> &'static str {
        match self {
            PipelineNode::Embed { .. } => "Embed",
            PipelineNode::Search { .. } => "Search",
            PipelineNode::Rerank { .. } => "Rerank",
            PipelineNode::Generate { .. } => "Generate",
            PipelineNode::Transform { .. } => "Transform",
            PipelineNode::Filter { .. } => "Filter",
            PipelineNode::Branch { .. } => "Branch",
            PipelineNode::Merge { .. } => "Merge",
            PipelineNode::Cache { .. } => "Cache",
        }
    }

    /// Returns `true` if this is a `Transform` node.
    pub fn is_transform(&self) -> bool {
        matches!(self, PipelineNode::Transform { .. })
    }

    /// Returns `true` if this is a `Cache` node.
    pub fn is_cache(&self) -> bool {
        matches!(self, PipelineNode::Cache { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_type_name() {
        let node = PipelineNode::Embed {
            model: "text-embedding-ada-002".into(),
            dimensions: 1536,
        };
        assert_eq!(node.type_name(), "Embed");
    }

    #[test]
    fn test_is_transform() {
        let t = PipelineNode::Transform {
            operation: "lowercase".into(),
        };
        assert!(t.is_transform());

        let e = PipelineNode::Embed {
            model: "m".into(),
            dimensions: 128,
        };
        assert!(!e.is_transform());
    }

    #[test]
    fn test_serde_roundtrip() {
        let node = PipelineNode::Generate {
            model: "gpt-4".into(),
            max_tokens: 1024,
            temperature: 0.7,
        };
        let json = serde_json::to_string(&node).unwrap();
        let back: PipelineNode = serde_json::from_str(&json).unwrap();
        assert_eq!(node, back);
    }
}
