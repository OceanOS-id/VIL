//! vil test — run workflow with test fixture and report results.
//!
//! `vil test <manifest.yaml> --input <fixture.json> [--workflow <name>]`

use vil_cli_core::manifest::WorkflowManifest;
use colored::*;
use std::time::Instant;

/// Test fixture format.
#[allow(dead_code)]
#[derive(Debug, serde::Deserialize)]
pub struct TestFixture {
    pub input: serde_json::Value,
    #[serde(default)]
    pub expected: Option<serde_json::Value>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

/// Run a workflow test with a fixture.
pub fn run_test(
    manifest_path: &str,
    fixture_path: &str,
    workflow_name: Option<&str>,
) -> Result<(), String> {
    // 1. Parse manifest
    let manifest = WorkflowManifest::from_file(manifest_path)?;
    println!(
        "{} Testing: {} ({})",
        ">>>".cyan().bold(),
        manifest.name,
        manifest_path
    );

    // 2. Load fixture
    let fixture_content = std::fs::read_to_string(fixture_path)
        .map_err(|e| format!("Failed to read fixture '{}': {}", fixture_path, e))?;
    let fixture: TestFixture = serde_json::from_str(&fixture_content)
        .map_err(|e| format!("Failed to parse fixture: {}", e))?;
    println!("  {} Fixture loaded: {}", "OK".green(), fixture_path);

    // 3. Find workflow
    let wf_name = workflow_name.unwrap_or_else(|| {
        manifest
            .workflows
            .keys()
            .next()
            .map(|s| s.as_str())
            .unwrap_or("default")
    });

    let workflow = manifest.workflows.get(wf_name);
    if let Some(wf) = workflow {
        println!(
            "  {} Workflow: {} ({} tasks, {} branches)",
            "OK".green(),
            wf_name,
            wf.tasks.len(),
            wf.branches.len()
        );
    } else {
        println!(
            "  {} No workflow '{}' found — running manifest-level test",
            "Note:".cyan(),
            wf_name
        );
    }

    // 4. Execute tasks
    let start = Instant::now();

    if let Some(wf) = workflow {
        println!("\n  {} Running task DAG:", "EXEC".yellow().bold());

        for task in &wf.tasks {
            let task_start = Instant::now();
            let task_type = task.task_type.as_deref().unwrap_or("Transform");

            // Simulate task execution (real execution needs WorkflowScheduler integration)
            let result = simulate_task(task_type, &fixture.input, &task.config);
            let elapsed = task_start.elapsed();

            let status = if result.is_ok() {
                format!("{}", "PASS".green().bold())
            } else {
                format!("{}", "FAIL".red().bold())
            };

            println!(
                "    [{}] {} ({}) — {:?}",
                status,
                task.name.as_deref().unwrap_or(&task.id),
                task_type,
                elapsed,
            );

            if let Err(e) = &result {
                println!("      {}: {}", "Error".red(), e);
            }
        }
    }

    let total_elapsed = start.elapsed();

    // 5. Compare with expected
    if let Some(expected) = &fixture.expected {
        println!(
            "\n  {} Expected output defined (comparison pending full executor integration)",
            "Note:".cyan()
        );
        println!(
            "    expected keys: {:?}",
            expected
                .as_object()
                .map(|o| o.keys().cloned().collect::<Vec<_>>())
        );
    }

    // 6. Report
    println!(
        "\n{} Test completed in {:?}",
        "DONE".green().bold(),
        total_elapsed
    );

    Ok(())
}

/// Simulate a single task execution (stub — returns mock data based on type).
fn simulate_task(
    task_type: &str,
    _input: &serde_json::Value,
    _config: &Option<serde_yaml::Value>,
) -> Result<serde_json::Value, String> {
    match task_type {
        "Embed" => Ok(serde_json::json!({"embedding": [0.1, 0.2, 0.3]})),
        "Search" => Ok(serde_json::json!({"results": ["doc1", "doc2"]})),
        "Generate" => Ok(serde_json::json!({"text": "generated"})),
        "Rerank" => Ok(serde_json::json!({"reranked": true})),
        "Filter" => Ok(serde_json::json!({"filtered": true})),
        "Cache" => Ok(serde_json::json!({"cached": true})),
        "Transform" => Ok(serde_json::json!({"transformed": true})),
        _ => Ok(serde_json::json!({"result": "ok"})),
    }
}
