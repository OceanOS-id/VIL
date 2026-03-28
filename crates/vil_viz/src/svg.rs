//! SVG output — shells out to `dot -Tsvg` from DOT output.

use crate::config::VizConfig;
use crate::graph::VizGraph;

pub fn render(graph: &VizGraph, config: &VizConfig) -> Result<String, String> {
    let dot_source = crate::dot::render(graph, config)?;

    // Try to use graphviz dot command
    let output = std::process::Command::new("dot")
        .args(["-Tsvg"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(stdin) = child.stdin.as_mut() {
                stdin.write_all(dot_source.as_bytes())?;
            }
            child.wait_with_output()
        });

    match output {
        Ok(out) if out.status.success() => String::from_utf8(out.stdout)
            .map_err(|e| format!("SVG output is not valid UTF-8: {}", e)),
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            Err(format!("dot command failed: {}", stderr))
        }
        Err(_) => {
            Err("Graphviz 'dot' not found. Install graphviz or use --format dot instead.".into())
        }
    }
}
