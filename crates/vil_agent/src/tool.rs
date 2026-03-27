//! Tool trait and ToolRegistry for agent tool-calling.

use async_trait::async_trait;
use std::sync::Arc;

/// Result of executing a tool.
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub output: String,
    pub metadata: Option<serde_json::Value>,
}

/// Errors that can occur during tool execution.
#[derive(Debug)]
pub enum ToolError {
    ExecutionFailed(String),
    InvalidParameters(String),
    Timeout,
}

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExecutionFailed(e) => write!(f, "tool execution failed: {}", e),
            Self::InvalidParameters(e) => write!(f, "invalid parameters: {}", e),
            Self::Timeout => write!(f, "tool execution timed out"),
        }
    }
}

impl std::error::Error for ToolError {}

/// Trait for tools that can be invoked by the agent.
///
/// Each tool exposes a name, description, and JSON Schema for its parameters,
/// which are sent to the LLM as function/tool definitions.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Unique name of the tool (used in LLM function calling).
    fn name(&self) -> &str;

    /// Human-readable description of what the tool does.
    fn description(&self) -> &str;

    /// JSON Schema describing the tool's parameters.
    fn parameters_schema(&self) -> serde_json::Value;

    /// Execute the tool with the given parameters.
    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult, ToolError>;
}

/// Registry of tools available to the agent.
pub struct ToolRegistry {
    tools: Vec<Arc<dyn Tool>>,
}

impl ToolRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    /// Register a tool.
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.push(tool);
    }

    /// Get a tool by name.
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Tool>> {
        self.tools.iter().find(|t| t.name() == name)
    }

    /// List all registered tool names.
    pub fn list(&self) -> Vec<&str> {
        self.tools.iter().map(|t| t.name()).collect()
    }

    /// Execute a tool by name with the given parameters.
    pub async fn execute(
        &self,
        name: &str,
        params: serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let tool = self
            .get(name)
            .ok_or_else(|| ToolError::ExecutionFailed(format!("tool '{}' not found", name)))?;
        tool.execute(params).await
    }

    /// Generate OpenAI-compatible tool definitions for LLM function calling.
    pub fn to_openai_tools(&self) -> Vec<serde_json::Value> {
        self.tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name(),
                        "description": t.description(),
                        "parameters": t.parameters_schema(),
                    }
                })
            })
            .collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyTool;

    #[async_trait]
    impl Tool for DummyTool {
        fn name(&self) -> &str {
            "example"
        }
        fn description(&self) -> &str {
            "An example tool for testing"
        }
        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "input": { "type": "string" }
                },
                "required": ["input"]
            })
        }
        async fn execute(&self, params: serde_json::Value) -> Result<ToolResult, ToolError> {
            let input = params["input"]
                .as_str()
                .ok_or(ToolError::InvalidParameters("missing input".into()))?;
            Ok(ToolResult {
                output: format!("echo: {}", input),
                metadata: None,
            })
        }
    }

    #[test]
    fn test_registry_register_and_get() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(DummyTool));
        assert!(registry.get("example").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_registry_list() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(DummyTool));
        let names = registry.list();
        assert_eq!(names, vec!["example"]);
    }

    #[test]
    fn test_to_openai_tools() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(DummyTool));
        let tools = registry.to_openai_tools();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["type"], "function");
        assert_eq!(tools[0]["function"]["name"], "example");
        assert_eq!(tools[0]["function"]["description"], "An example tool for testing");
        assert!(tools[0]["function"]["parameters"]["properties"]["input"].is_object());
    }

    #[tokio::test]
    async fn test_registry_execute() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(DummyTool));
        let result = registry
            .execute("example", serde_json::json!({"input": "hello"}))
            .await
            .unwrap();
        assert_eq!(result.output, "echo: hello");
    }

    #[tokio::test]
    async fn test_registry_execute_not_found() {
        let registry = ToolRegistry::new();
        let result = registry
            .execute("nonexistent", serde_json::json!({}))
            .await;
        assert!(result.is_err());
    }
}
