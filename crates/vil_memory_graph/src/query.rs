use crate::entity::Entity;
use crate::graph::MemoryGraph;

/// Find all entities reachable within `max_depth` hops from `start_id`.
pub fn find_related(graph: &MemoryGraph, start_id: u64, max_depth: usize) -> Vec<Entity> {
    let mut visited = std::collections::HashSet::new();
    let mut frontier = vec![start_id];
    visited.insert(start_id);

    for _ in 0..max_depth {
        let mut next_frontier = Vec::new();
        for id in &frontier {
            for neighbor in graph.neighbors(*id) {
                if visited.insert(neighbor.id) {
                    next_frontier.push(neighbor.id);
                }
            }
        }
        if next_frontier.is_empty() {
            break;
        }
        frontier = next_frontier;
    }

    // Remove the start node itself from results.
    visited.remove(&start_id);
    let entities = graph.entities_ref().read();
    visited
        .iter()
        .filter_map(|id| entities.iter().find(|e| e.id == *id).cloned())
        .collect()
}

/// Find the shortest path (BFS) between two entities. Returns entity IDs in order,
/// or `None` if no path exists.
pub fn shortest_path(graph: &MemoryGraph, from: u64, to: u64) -> Option<Vec<u64>> {
    if from == to {
        return Some(vec![from]);
    }

    let mut visited = std::collections::HashSet::new();
    let mut queue = std::collections::VecDeque::new();
    // Each entry: (current_id, path_so_far)
    queue.push_back((from, vec![from]));
    visited.insert(from);

    while let Some((current, path)) = queue.pop_front() {
        for neighbor in graph.neighbors(current) {
            if visited.contains(&neighbor.id) {
                continue;
            }
            let mut new_path = path.clone();
            new_path.push(neighbor.id);
            if neighbor.id == to {
                return Some(new_path);
            }
            visited.insert(neighbor.id);
            queue.push_back((neighbor.id, new_path));
        }
    }

    None
}
