//! VIL Multi-Agent System — compile agent graphs to state machines with typed
//! inter-agent channels.
//!
//! This crate provides the building blocks for orchestrating multiple AI agents
//! that collaborate through a directed acyclic graph (DAG). Each agent is a node
//! in the graph, and directed edges define how output flows from one agent to
//! another via typed message channels.
//!
//! # Architecture
//!
//! - **AgentNode** — wrapper around any `AgentRunnable` with a name, role, and
//!   edge metadata (upstream / downstream).
//! - **AgentGraph** — DAG of agent nodes with a fluent builder API.
//! - **AgentChannel** — typed tokio MPSC channel carrying `AgentMessage`s.
//! - **Orchestrator** — drives execution: starts at root agents, passes output
//!   downstream, collects final answers from leaf agents.
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use vil_multi_agent::{AgentGraph, Orchestrator};
//!
//! let graph = AgentGraph::builder()
//!     .agent("planner", planner_agent)
//!     .agent("executor", executor_agent)
//!     .agent("reviewer", reviewer_agent)
//!     .edge("planner", "executor")
//!     .edge("executor", "reviewer")
//!     .build()
//!     .unwrap();
//!
//! let mut orchestrator = Orchestrator::new(graph);
//! let result = orchestrator.run("Design a REST API").await.unwrap();
//! println!("Final answer: {}", result.final_answer);
//! ```

pub mod agent_node;
pub mod channel;
pub mod config;
pub mod graph;
pub mod orchestrator;
pub mod semantic;
pub mod pipeline_sse;
pub mod handlers;
pub mod plugin;

pub use agent_node::{AgentNode, AgentRunnable};
pub use channel::{AgentChannel, AgentMessage};
pub use config::MultiAgentConfig;
pub use graph::{AgentGraph, AgentGraphBuilder, GraphError};
pub use orchestrator::{Orchestrator, OrchestratorError, OrchestratorResult};
pub use plugin::MultiAgentPlugin;
pub use semantic::{MultiAgentEvent, MultiAgentFault, MultiAgentState};
