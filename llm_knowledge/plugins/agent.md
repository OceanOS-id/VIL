# Agent Plugin

`vil_agent` provides tool-augmented AI agents with ReAct loop execution.

## Basic Agent

```rust
use vil_agent::prelude::*;

let agent = Agent::new()
    .llm(LlmProvider::openai().model("gpt-4o").build())
    .tool(CalculatorTool::new())
    .tool(HttpFetchTool::new())
    .max_turns(10)
    .build();

let result = agent.run("What is the population of Tokyo divided by 3?").await?;
println!("{}", result.final_answer);
```

## Built-in Tools

| Tool | Description |
|------|-------------|
| `CalculatorTool` | Math expressions (safe eval) |
| `HttpFetchTool` | HTTP GET with response parsing |
| `RetrievalTool` | Vector search against RAG store |

## Custom Tool

```rust
use vil_agent::tool::{Tool, ToolResult};

struct WeatherTool;

impl Tool for WeatherTool {
    fn name(&self) -> &str { "weather" }
    fn description(&self) -> &str { "Get current weather for a city" }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "city": { "type": "string", "description": "City name" }
            },
            "required": ["city"]
        })
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let city = params["city"].as_str().unwrap_or("unknown");
        let weather = fetch_weather(city).await?;
        ToolResult::ok(serde_json::to_value(weather)?)
    }
}
```

## ReAct Loop

The agent follows Thought-Action-Observation cycles:

```
Thought: I need to find Tokyo's population, then divide by 3.
Action: http_fetch(url="https://api.example.com/population?city=Tokyo")
Observation: {"population": 13960000}
Thought: Now I divide 13960000 by 3.
Action: calculator(expression="13960000 / 3")
Observation: 4653333.33
Thought: I have the answer.
Final Answer: Tokyo's population divided by 3 is approximately 4,653,333.
```

## As VilPlugin

```rust
use vil_agent::AgentPlugin;

VilApp::new("agent-service")
    .port(8080)
    .plugin(AgentPlugin::new()
        .llm(provider)
        .tool(CalculatorTool::new())
        .tool(HttpFetchTool::new()))
    .service(api_service)
    .run()
    .await;
```

## Server Handler

```rust
#[vil_handler(shm)]
async fn run_agent(ctx: ServiceCtx, slice: ShmSlice) -> VilResponse<AgentResult> {
    let input: AgentQuery = slice.json()?;
    let agent = ctx.state::<Agent>();
    let result = agent.run(&input.prompt).await?;
    VilResponse::ok(result)
}
```

## Agent Memory

```rust
let agent = Agent::new()
    .llm(provider)
    .memory(ConversationMemory::new(max_turns: 20))
    .build();

// Multi-turn conversation
let r1 = agent.run("My name is Alice.").await?;
let r2 = agent.run("What is my name?").await?;  // Remembers "Alice"
```

> Reference: docs/vil/005-VIL-Developer_Guide-Plugins-AI.md
