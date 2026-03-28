//! ReAct agent with tool-calling loop.

use std::sync::Arc;
use serde::Serialize;
use vil_llm::{ChatMessage, LlmProvider};
use vil_llm::message::LlmError;
use vil_log::app_log;

use crate::memory::ConversationMemory;
use crate::tool::{Tool, ToolError, ToolRegistry};

/// Error from agent execution.
#[derive(Debug)]
pub enum AgentError {
    LlmError(LlmError),
    ToolError(ToolError),
    MaxIterationsReached(usize),
}

impl std::fmt::Display for AgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LlmError(e) => write!(f, "LLM error: {}", e),
            Self::ToolError(e) => write!(f, "tool error: {}", e),
            Self::MaxIterationsReached(n) => {
                write!(f, "max iterations reached ({})", n)
            }
        }
    }
}

impl std::error::Error for AgentError {}

impl From<LlmError> for AgentError {
    fn from(e: LlmError) -> Self {
        Self::LlmError(e)
    }
}

impl From<ToolError> for AgentError {
    fn from(e: ToolError) -> Self {
        Self::ToolError(e)
    }
}

/// Record of a single tool call made during agent execution.
#[derive(Debug, Clone, Serialize)]
pub struct AgentToolCall {
    pub tool: String,
    pub input: serde_json::Value,
    pub output: String,
}

/// The agent's final response after completing the ReAct loop.
#[derive(Debug, Clone, Serialize)]
pub struct AgentResponse {
    pub answer: String,
    pub tool_calls_made: Vec<AgentToolCall>,
    pub iterations: usize,
}

/// AI agent with ReAct loop and tool-calling capability.
///
/// The agent sends messages + tool definitions to the LLM. If the LLM returns
/// tool calls, they are executed and results fed back. This continues until the
/// LLM produces a final text answer or max iterations are reached.
pub struct Agent {
    llm: Arc<dyn LlmProvider>,
    tools: ToolRegistry,
    memory: ConversationMemory,
    max_iterations: usize,
    #[allow(dead_code)]
    system_prompt: String,
}

impl Agent {
    /// Create an AgentBuilder.
    pub fn builder() -> AgentBuilder {
        AgentBuilder::new()
    }

    /// Get a reference to the conversation memory.
    pub fn memory(&self) -> &ConversationMemory {
        &self.memory
    }

    /// Get the list of registered tool names.
    pub fn tool_names(&self) -> Vec<String> {
        self.tools.list().iter().map(|s| s.to_string()).collect()
    }

    /// Run the agent with a user query.
    ///
    /// Executes the ReAct loop:
    /// 1. Send messages + tool definitions to LLM
    /// 2. If LLM returns tool_calls -> execute tools -> add results to context -> goto 1
    /// 3. If LLM returns text (no tool calls) -> return as final answer
    /// 4. If max_iterations exceeded -> return error
    pub async fn run(&self, query: &str) -> Result<AgentResponse, AgentError> {
        // Add the user query to memory
        self.memory.add(ChatMessage::user(query)).await;

        let tool_defs = self.tools.to_openai_tools();
        let mut tool_calls_made = Vec::new();
        let mut iterations = 0;

        loop {
            iterations += 1;
            if iterations > self.max_iterations {
                return Err(AgentError::MaxIterationsReached(self.max_iterations));
            }

            let context = self.memory.get_context().await;

            app_log!(Debug, "agent_react", { iteration: iterations, context_len: context.len() });

            // Call LLM with tools
            let response = self
                .llm
                .chat_with_tools(&context, &tool_defs)
                .await
                .map_err(AgentError::LlmError)?;

            // Check if LLM wants to call tools
            match response.tool_calls {
                Some(ref calls) if !calls.is_empty() => {
                    // Add assistant message to memory
                    self.memory
                        .add(ChatMessage::assistant(&response.content))
                        .await;

                    // Execute each tool call
                    for call in calls {
                        app_log!(Info, "agent_tool_call", { tool: call.name.clone(), id: call.id.clone() });

                        let result = self
                            .tools
                            .execute(&call.name, call.arguments.clone())
                            .await;

                        let output = match result {
                            Ok(r) => r.output,
                            Err(e) => format!("Error: {}", e),
                        };

                        tool_calls_made.push(AgentToolCall {
                            tool: call.name.clone(),
                            input: call.arguments.clone(),
                            output: output.clone(),
                        });

                        // Add tool result to memory
                        self.memory
                            .add(ChatMessage::tool_result(&call.id, &output))
                            .await;
                    }
                }
                _ => {
                    // No tool calls -- this is the final answer
                    let answer = response.content.clone();
                    self.memory.add(ChatMessage::assistant(&answer)).await;

                    return Ok(AgentResponse {
                        answer,
                        tool_calls_made,
                        iterations,
                    });
                }
            }
        }
    }
}

/// Builder for constructing an Agent with a fluent API.
pub struct AgentBuilder {
    llm: Option<Arc<dyn LlmProvider>>,
    tools: Vec<Arc<dyn Tool>>,
    max_iterations: usize,
    system_prompt: String,
    memory_size: usize,
}

impl AgentBuilder {
    pub fn new() -> Self {
        Self {
            llm: None,
            tools: Vec::new(),
            max_iterations: 10,
            system_prompt: "You are a helpful AI assistant with access to tools. Use tools when needed to answer the user's question accurately.".into(),
            memory_size: 50,
        }
    }

    /// Set the LLM provider.
    pub fn llm(mut self, llm: Arc<dyn LlmProvider>) -> Self {
        self.llm = Some(llm);
        self
    }

    /// Add a tool to the agent.
    pub fn tool(mut self, tool: Arc<dyn Tool>) -> Self {
        self.tools.push(tool);
        self
    }

    /// Set the maximum number of ReAct loop iterations.
    pub fn max_iterations(mut self, n: usize) -> Self {
        self.max_iterations = n;
        self
    }

    /// Set the system prompt.
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = prompt.into();
        self
    }

    /// Set the conversation memory window size.
    pub fn memory_size(mut self, n: usize) -> Self {
        self.memory_size = n;
        self
    }

    /// Build the agent. Panics if no LLM provider is set.
    pub fn build(self) -> Agent {
        let llm = self
            .llm
            .expect("AgentBuilder requires an LLM provider (.llm())");

        let mut tools = ToolRegistry::new();
        for t in self.tools {
            tools.register(t);
        }

        let memory =
            ConversationMemory::new(self.memory_size).with_system_prompt(&self.system_prompt);

        Agent {
            llm,
            tools,
            memory,
            max_iterations: self.max_iterations,
            system_prompt: self.system_prompt,
        }
    }
}

impl Default for AgentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::{Tool, ToolResult};
    use async_trait::async_trait;
    use vil_llm::ChatResponse;
    use vil_llm::message::LlmError;

    /// A no-op LLM that returns a fixed response with no tool calls.
    struct MockLlm;

    #[async_trait]
    impl LlmProvider for MockLlm {
        async fn chat(&self, _messages: &[ChatMessage]) -> Result<ChatResponse, LlmError> {
            Ok(ChatResponse {
                content: "I am a no-op LLM response.".into(),
                model: "noop".into(),
                tool_calls: None,
                usage: None,
                finish_reason: Some("stop".into()),
            })
        }
        fn model(&self) -> &str {
            "noop"
        }
        fn provider_name(&self) -> &str {
            "noop"
        }
    }

    struct EchoTool;

    #[async_trait]
    impl Tool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }
        fn description(&self) -> &str {
            "Echoes input"
        }
        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({"type": "object", "properties": {"text": {"type": "string"}}})
        }
        async fn execute(
            &self,
            params: serde_json::Value,
        ) -> Result<ToolResult, crate::tool::ToolError> {
            Ok(ToolResult {
                output: params["text"].as_str().unwrap_or("").to_string(),
                metadata: None,
            })
        }
    }

    #[test]
    fn test_builder_builds_agent() {
        let agent = Agent::builder()
            .llm(Arc::new(MockLlm))
            .tool(Arc::new(EchoTool))
            .max_iterations(5)
            .system_prompt("Test prompt")
            .memory_size(20)
            .build();

        assert_eq!(agent.max_iterations, 5);
        assert_eq!(agent.system_prompt, "Test prompt");
        assert_eq!(agent.tools.list(), vec!["echo"]);
    }

    #[tokio::test]
    async fn test_agent_run_no_tools() {
        let agent = Agent::builder()
            .llm(Arc::new(MockLlm))
            .build();

        let response = agent.run("hello").await.unwrap();
        assert_eq!(response.answer, "I am a no-op LLM response.");
        assert!(response.tool_calls_made.is_empty());
        assert_eq!(response.iterations, 1);
    }

    #[test]
    #[should_panic(expected = "AgentBuilder requires an LLM provider")]
    fn test_builder_panics_without_llm() {
        Agent::builder().build();
    }
}
