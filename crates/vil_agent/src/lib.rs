//! VIL Agent Plugin — tool-calling AI agent with ReAct loop.
//!
//! Provides an `Agent` that iteratively calls tools via LLM function-calling
//! until it arrives at a final answer. Registers as a VIL plugin via
//! `VilApp::new("app").plugin(AgentPlugin::new())`.
//!
//! Depends on `vil_llm` (required) and `vil_rag` (optional — auto-adds
//! retrieval tool if RAG plugin is registered).
//!
//! # Plugin endpoints
//!
//! | Method | Path | Description |
//! |--------|------|-------------|
//! | POST | /api/agent/run | Execute agent query |
//! | GET | /api/agent/tools | List available tools |
//! | POST | /api/agent/memory/clear | Clear conversation memory |

pub mod agent;
pub mod memory;
pub mod tool;
pub mod tools;
pub mod semantic;
pub mod extractors;
pub mod pipeline_sse;
pub mod handlers;
pub mod plugin;

pub use agent::{Agent, AgentBuilder, AgentError, AgentResponse, AgentToolCall};
pub use memory::ConversationMemory;
pub use tool::{Tool, ToolRegistry, ToolResult, ToolError};
pub use tools::{CalculatorTool, HttpFetchTool, RetrievalTool};
pub use plugin::AgentPlugin;
pub use extractors::AgentHandle;
pub use semantic::{
    AgentToolCallEvent, AgentCompletionEvent, AgentFault, AgentFaultType,
    AgentMemoryState, AgentRoutingDecision,
};
