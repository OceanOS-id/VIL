//! VilPlugin implementation for the Agent framework.
//!
//! Depends on vil-llm (required) and vil-rag (optional).
//! Registers ServiceProcess with /run, /tools, /memory/clear endpoints.

use vil_server::prelude::*;

use std::sync::Arc;

use vil_llm::LlmProvider;
use vil_rag::Retriever;

use crate::agent::Agent;
use crate::extractors::AgentHandle;
use crate::handlers::{self, AgentServiceState, ToolInfo, ToolsResponseBody};
use crate::semantic::{AgentCompletionEvent, AgentFault, AgentMemoryState};
use crate::tool::Tool;
use crate::tools::RetrievalTool;

/// VIL Agent Plugin — tool-calling AI agent with ReAct loop.
///
/// # Example
/// ```ignore
/// VilApp::new("agent-service")
///     .plugin(LlmPlugin::new().openai(config))
///     .plugin(RagPlugin::new())
///     .plugin(
///         AgentPlugin::new()
///             .tool(Arc::new(CalculatorTool))
///             .tool(Arc::new(HttpFetchTool::new()))
///             .max_iterations(15)
///     )
///     .run().await;
/// ```
pub struct AgentPlugin {
    tools: Vec<Arc<dyn Tool>>,
    max_iterations: usize,
    system_prompt: String,
}

impl AgentPlugin {
    pub fn new() -> Self {
        Self {
            tools: Vec::new(),
            max_iterations: 10,
            system_prompt: "You are a helpful AI assistant with access to tools.".into(),
        }
    }

    pub fn tool(mut self, tool: Arc<dyn Tool>) -> Self {
        self.tools.push(tool);
        self
    }

    pub fn max_iterations(mut self, n: usize) -> Self {
        self.max_iterations = n;
        self
    }

    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = prompt.into();
        self
    }
}

impl Default for AgentPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl VilPlugin for AgentPlugin {
    fn id(&self) -> &str {
        "vil-agent"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn description(&self) -> &str {
        "AI agent framework with tool-calling and ReAct loop"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![
            PluginCapability::Resource {
                type_name: "Agent",
                name: "agent".into(),
            },
            PluginCapability::Service {
                name: "agent".into(),
                endpoints: vec![
                    EndpointSpec::post("/api/agent/run").with_description("Run agent query"),
                    EndpointSpec::get("/api/agent/tools").with_description("List available tools"),
                    EndpointSpec::post("/api/agent/memory/clear")
                        .with_description("Clear conversation memory"),
                ],
            },
        ]
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        vec![
            PluginDependency::required("vil-llm", ">=0.1"),
            PluginDependency::optional("vil-rag", ">=0.1"),
        ]
    }

    fn register(&self, ctx: &mut PluginContext) {
        let llm = ctx.require::<Arc<dyn LlmProvider>>("llm").clone();

        // Auto-discover RAG retriever if available
        let mut tools = self.tools.clone();
        if let Some(retriever) = ctx.get::<Arc<dyn Retriever>>("rag-retriever") {
            tools.push(Arc::new(RetrievalTool::new(retriever.clone())));
        }

        // Build tool info list for /tools endpoint
        let tool_info: Vec<ToolInfo> = tools
            .iter()
            .map(|t| ToolInfo {
                name: t.name().to_string(),
                description: t.description().to_string(),
            })
            .collect();
        let tools_resp = ToolsResponseBody {
            count: tool_info.len(),
            tools: tool_info,
        };

        // Build agent
        let builder = Agent::builder()
            .llm(llm)
            .system_prompt(&self.system_prompt)
            .max_iterations(self.max_iterations);
        let builder = tools.into_iter().fold(builder, |b, t| b.tool(t));
        let agent = Arc::new(builder.build());

        // Provide agent resource
        ctx.provide::<Arc<Agent>>("agent", agent.clone());

        // Build ServiceProcess
        let svc = ServiceProcess::new("agent")
            .endpoint(Method::POST, "/run", post(handlers::run_handler))
            .endpoint(Method::GET, "/tools", get(handlers::tools_handler))
            .endpoint(
                Method::POST,
                "/memory/clear",
                post(handlers::clear_memory_handler),
            )
            .state(AgentServiceState {
                agent: AgentHandle::from(agent),
                tools_resp,
            })
            .emits::<AgentCompletionEvent>()
            .faults::<AgentFault>()
            .manages::<AgentMemoryState>();

        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
