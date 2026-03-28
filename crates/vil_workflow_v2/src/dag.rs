use std::collections::{HashMap, VecDeque};

use crate::task::Task;

/// Resolves task execution order from a DAG of tasks.
/// Returns layers of task IDs that can be executed in parallel within each layer.
pub fn resolve_layers(tasks: &[Task]) -> Result<Vec<Vec<String>>, String> {
    if tasks.is_empty() {
        return Ok(Vec::new());
    }

    let task_map: HashMap<&str, &Task> = tasks.iter().map(|t| (t.id.as_str(), t)).collect();

    // Validate all deps exist
    for task in tasks {
        for dep in &task.deps {
            if !task_map.contains_key(dep.as_str()) {
                return Err(format!(
                    "Task '{}' depends on unknown task '{}'",
                    task.id, dep
                ));
            }
        }
    }

    // Kahn's algorithm for topological sort into layers
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();

    for task in tasks {
        in_degree.entry(task.id.as_str()).or_insert(0);
        for dep in &task.deps {
            *in_degree.entry(task.id.as_str()).or_insert(0) += 1;
            dependents
                .entry(dep.as_str())
                .or_default()
                .push(task.id.as_str());
        }
    }

    let mut layers: Vec<Vec<String>> = Vec::new();
    let mut queue: VecDeque<String> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&id, _)| id.to_string())
        .collect();

    let mut total_processed = 0usize;

    while !queue.is_empty() {
        let layer: Vec<String> = queue.drain(..).collect();
        let mut next_queue = VecDeque::new();

        for id in &layer {
            if let Some(deps) = dependents.get(id.as_str()) {
                for &dep_id in deps {
                    if let Some(deg) = in_degree.get_mut(dep_id) {
                        *deg -= 1;
                        if *deg == 0 {
                            next_queue.push_back(dep_id.to_string());
                        }
                    }
                }
            }
        }

        total_processed += layer.len();
        layers.push(layer);
        queue = next_queue;
    }
    if total_processed != tasks.len() {
        return Err("Cycle detected in task dependencies".to_string());
    }

    Ok(layers)
}
