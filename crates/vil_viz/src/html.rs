//! Self-contained HTML output with embedded Mermaid.js for interactive viewing.

use crate::config::VizConfig;
use crate::graph::VizGraph;

pub fn render(graph: &VizGraph, config: &VizConfig) -> Result<String, String> {
    let mermaid_source = crate::mermaid::render(graph, config)?;
    let json_source = crate::json::render(graph)?;

    // Escape for embedding in HTML
    let mermaid_escaped = mermaid_source
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");

    Ok(format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>VIL Workflow: {name}</title>
<style>
  body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; margin: 0; padding: 20px; background: #1a1a2e; color: #eee; }}
  h1 {{ font-size: 1.4em; color: #7fdbca; margin-bottom: 4px; }}
  .meta {{ font-size: 0.85em; color: #888; margin-bottom: 20px; }}
  .mermaid {{ background: #16213e; padding: 20px; border-radius: 8px; overflow-x: auto; }}
  details {{ margin-top: 20px; }}
  summary {{ cursor: pointer; color: #7fdbca; font-size: 0.9em; }}
  pre {{ background: #0f3460; padding: 12px; border-radius: 6px; overflow-x: auto; font-size: 0.8em; color: #ccc; }}
</style>
</head>
<body>
<h1>VIL Workflow: {name}</h1>
<div class="meta">{nodes} nodes, {edges} routes{subgraphs}</div>

<div class="mermaid">
{mermaid}
</div>

<details>
<summary>JSON Graph Data (for IDE / egui)</summary>
<pre id="json-data">{json}</pre>
</details>

<details>
<summary>Mermaid Source</summary>
<pre>{mermaid_src}</pre>
</details>

<script src="https://cdn.jsdelivr.net/npm/mermaid@10/dist/mermaid.min.js"></script>
<script>
mermaid.initialize({{ startOnLoad: true, theme: 'dark', securityLevel: 'loose' }});
</script>
</body>
</html>"#,
        name = graph.name,
        nodes = graph.nodes.len(),
        edges = graph.edges.len(),
        subgraphs = if graph.subgraphs.is_empty() {
            String::new()
        } else {
            format!(", {} workflow DAGs", graph.subgraphs.len())
        },
        mermaid = mermaid_source,
        json = json_source,
        mermaid_src = mermaid_escaped,
    ))
}
