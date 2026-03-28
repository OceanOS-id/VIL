//! RAG retrieval tool — searches the knowledge base via the Retriever trait.

use async_trait::async_trait;
use std::sync::Arc;
use vil_rag::Retriever;

use crate::tool::{Tool, ToolError, ToolResult};

/// Tool that searches a knowledge base using the RAG retriever.
pub struct RetrievalTool {
    retriever: Arc<dyn Retriever>,
    top_k: usize,
}

impl RetrievalTool {
    /// Create a new retrieval tool with the given retriever.
    pub fn new(retriever: Arc<dyn Retriever>) -> Self {
        Self {
            retriever,
            top_k: 5,
        }
    }

    /// Set the number of results to retrieve.
    pub fn with_top_k(mut self, k: usize) -> Self {
        self.top_k = k;
        self
    }
}

#[async_trait]
impl Tool for RetrievalTool {
    fn name(&self) -> &str {
        "search_knowledge_base"
    }

    fn description(&self) -> &str {
        "Search the knowledge base for relevant documents"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult, ToolError> {
        let query = params["query"]
            .as_str()
            .ok_or(ToolError::InvalidParameters("missing query".into()))?;

        let results = self
            .retriever
            .retrieve(query, self.top_k)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let output = results
            .iter()
            .enumerate()
            .map(|(i, r)| format!("[{}] (score: {:.2}) {}", i + 1, r.score, r.content))
            .collect::<Vec<_>>()
            .join("\n\n");

        Ok(ToolResult {
            output,
            metadata: None,
        })
    }
}
