//! Compiler: optimize a `PipelineDag` into a `CompiledPlan`.

use crate::dag::{DagError, PipelineDag};
use crate::node::PipelineNode;

use serde::{Deserialize, Serialize};

/// A single step in a compiled execution plan.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionStep {
    /// Node identifier.
    pub node_id: String,
    /// The pipeline node definition.
    pub node_type: PipelineNode,
    /// Ids of nodes this step depends on.
    pub dependencies: Vec<String>,
    /// Ids of nodes that can execute in parallel with this step.
    pub can_parallel_with: Vec<String>,
}

/// A compiled, optimized execution plan.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompiledPlan {
    /// Steps in topological order.
    pub steps: Vec<ExecutionStep>,
    /// Groups of step indices that can run in parallel.
    /// Each inner `Vec` is one parallel tier.
    pub parallelizable: Vec<Vec<usize>>,
}

impl CompiledPlan {
    /// Total number of steps.
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// Number of parallel tiers.
    pub fn tier_count(&self) -> usize {
        self.parallelizable.len()
    }
}

/// Compile a `PipelineDag` into an optimized `CompiledPlan`.
///
/// Optimisations applied:
/// 1. Topological sort for correct execution order.
/// 2. Parallel-group identification (nodes whose deps are all satisfied).
/// 3. Fuse consecutive `Transform` nodes into a single combined transform.
/// 4. Detect and eliminate redundant `Cache` nodes (cache right before an
///    identical cache).
pub fn compile(dag: &PipelineDag) -> Result<CompiledPlan, DagError> {
    let n = dag.nodes.len();
    if n == 0 {
        return Ok(CompiledPlan {
            steps: vec![],
            parallelizable: vec![],
        });
    }

    // --- 1. Topological sort (Kahn's) ---
    let mut in_degree = vec![0usize; n];
    for &(_, to) in &dag.edges {
        in_degree[to] += 1;
    }

    let mut queue: Vec<usize> = in_degree
        .iter()
        .enumerate()
        .filter_map(|(i, &d)| if d == 0 { Some(i) } else { None })
        .collect();

    // Sort the initial queue for deterministic output.
    queue.sort();

    let mut topo_order: Vec<usize> = Vec::with_capacity(n);
    let mut tiers: Vec<Vec<usize>> = Vec::new();

    // BFS-style layer-by-layer for parallel tiers.
    while !queue.is_empty() {
        queue.sort();
        let current_tier = queue.clone();
        tiers.push(current_tier.clone());

        let mut next_queue = Vec::new();
        for &node in &current_tier {
            topo_order.push(node);
            for &(from, to) in &dag.edges {
                if from == node {
                    in_degree[to] -= 1;
                    if in_degree[to] == 0 {
                        next_queue.push(to);
                    }
                }
            }
        }
        queue = next_queue;
    }

    if topo_order.len() != n {
        return Err(DagError::CycleDetected);
    }

    // --- 3. Fuse consecutive Transforms ---
    // Identify chains: a Transform whose sole successor is also a Transform
    // with no other predecessors. `fused_into[x] = Some(root)` means node x
    // is absorbed into the chain headed by `root`.
    let mut fused_into: Vec<Option<usize>> = vec![None; n];
    for &idx in &topo_order {
        if !dag.nodes[idx].1.is_transform() {
            continue;
        }
        // Determine the root of the chain this node belongs to.
        let root = fused_into[idx].unwrap_or(idx);
        let succs = dag.successors(idx);
        if succs.len() == 1 {
            let succ = succs[0];
            if dag.nodes[succ].1.is_transform() && dag.predecessors(succ).len() == 1 {
                fused_into[succ] = Some(root);
            }
        }
    }

    // --- 4. Eliminate redundant Cache nodes ---
    // A Cache node is redundant if its sole predecessor is also a Cache node
    // with the same key_expr.
    let mut redundant: Vec<bool> = vec![false; n];
    for &idx in &topo_order {
        if let PipelineNode::Cache { key_expr, .. } = &dag.nodes[idx].1 {
            let preds = dag.predecessors(idx);
            if preds.len() == 1 {
                if let PipelineNode::Cache {
                    key_expr: prev_key, ..
                } = &dag.nodes[preds[0]].1
                {
                    if prev_key == key_expr {
                        redundant[idx] = true;
                    }
                }
            }
        }
    }

    // --- Build steps, skipping fused / redundant nodes ---
    // Map from original index to step index.
    let mut idx_to_step: Vec<Option<usize>> = vec![None; n];
    let mut steps: Vec<ExecutionStep> = Vec::new();

    for &idx in &topo_order {
        if fused_into[idx].is_some() || redundant[idx] {
            continue;
        }

        // Collect fused operation names.
        let node_type = if dag.nodes[idx].1.is_transform() {
            let mut ops = Vec::new();
            if let PipelineNode::Transform { operation } = &dag.nodes[idx].1 {
                ops.push(operation.clone());
            }
            // Walk the fuse chain forward: find successors fused into this root.
            let mut cur = idx;
            loop {
                let succs = dag.successors(cur);
                // Find a successor that was fused into this root.
                let fused_child = succs.into_iter().find(|&s| fused_into[s] == Some(idx));
                match fused_child {
                    Some(child) => {
                        if let PipelineNode::Transform { operation } = &dag.nodes[child].1 {
                            ops.push(operation.clone());
                        }
                        cur = child;
                    }
                    None => break,
                }
            }
            if ops.len() > 1 {
                PipelineNode::Transform {
                    operation: ops.join(" | "),
                }
            } else {
                dag.nodes[idx].1.clone()
            }
        } else {
            dag.nodes[idx].1.clone()
        };

        let dep_ids: Vec<String> = dag
            .predecessors(idx)
            .iter()
            .filter_map(|&p| {
                if let Some(step_i) = idx_to_step[p] {
                    Some(steps[step_i].node_id.clone())
                } else {
                    // predecessor was fused/redundant — walk to root
                    let mut root = p;
                    while let Some(pr) = fused_into[root] {
                        root = pr;
                    }
                    idx_to_step[root].map(|si| steps[si].node_id.clone())
                }
            })
            .collect();

        let step_idx = steps.len();
        idx_to_step[idx] = Some(step_idx);

        steps.push(ExecutionStep {
            node_id: dag.nodes[idx].0.clone(),
            node_type,
            dependencies: dep_ids,
            can_parallel_with: vec![], // filled below
        });
    }

    // --- Rebuild parallel tiers based on remaining steps ---
    // Re-derive tiers: step belongs to the tier just after the latest tier of
    // any of its dependencies.
    let mut step_tier: Vec<usize> = vec![0; steps.len()];
    for (si, step) in steps.iter().enumerate() {
        let max_dep_tier = step
            .dependencies
            .iter()
            .filter_map(|dep_id| {
                steps
                    .iter()
                    .position(|s| s.node_id == *dep_id)
                    .map(|di| step_tier[di])
            })
            .max();
        step_tier[si] = match max_dep_tier {
            Some(t) => t + 1,
            None => 0,
        };
    }

    let max_tier = step_tier.iter().copied().max().unwrap_or(0);
    let mut parallel_groups: Vec<Vec<usize>> = Vec::new();
    for t in 0..=max_tier {
        let group: Vec<usize> = step_tier
            .iter()
            .enumerate()
            .filter_map(|(si, &st)| if st == t { Some(si) } else { None })
            .collect();
        if !group.is_empty() {
            parallel_groups.push(group);
        }
    }

    // Fill can_parallel_with.
    for group in &parallel_groups {
        for &si in group {
            let others: Vec<String> = group
                .iter()
                .filter(|&&oi| oi != si)
                .map(|&oi| steps[oi].node_id.clone())
                .collect();
            steps[si].can_parallel_with = others;
        }
    }

    Ok(CompiledPlan {
        steps,
        parallelizable: parallel_groups,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::DagBuilder;
    use crate::node::{MergeStrategy, PipelineNode};

    fn embed() -> PipelineNode {
        PipelineNode::Embed {
            model: "ada".into(),
            dimensions: 1536,
        }
    }
    fn search() -> PipelineNode {
        PipelineNode::Search {
            index: "docs".into(),
            top_k: 10,
        }
    }
    fn generate() -> PipelineNode {
        PipelineNode::Generate {
            model: "gpt-4".into(),
            max_tokens: 1024,
            temperature: 0.7,
        }
    }
    fn transform(op: &str) -> PipelineNode {
        PipelineNode::Transform {
            operation: op.into(),
        }
    }
    fn cache(key: &str) -> PipelineNode {
        PipelineNode::Cache {
            ttl_secs: 60,
            key_expr: key.into(),
        }
    }
    fn merge() -> PipelineNode {
        PipelineNode::Merge {
            strategy: MergeStrategy::Concat,
        }
    }
    fn rerank() -> PipelineNode {
        PipelineNode::Rerank {
            model: "reranker".into(),
            top_n: 5,
        }
    }
    fn filter() -> PipelineNode {
        PipelineNode::Filter {
            predicate: "score > 0.5".into(),
        }
    }

    #[test]
    fn test_empty_dag() {
        let dag = DagBuilder::new().build().unwrap();
        let plan = compile(&dag).unwrap();
        assert_eq!(plan.step_count(), 0);
        assert_eq!(plan.tier_count(), 0);
    }

    #[test]
    fn test_single_node() {
        let dag = DagBuilder::new().node("e", embed()).build().unwrap();
        let plan = compile(&dag).unwrap();
        assert_eq!(plan.step_count(), 1);
        assert_eq!(plan.steps[0].node_id, "e");
        assert!(plan.steps[0].dependencies.is_empty());
    }

    #[test]
    fn test_linear_dag() {
        // embed -> search -> generate
        let dag = DagBuilder::new()
            .node("embed", embed())
            .node("search", search())
            .node("generate", generate())
            .edge("embed", "search")
            .edge("search", "generate")
            .build()
            .unwrap();

        let plan = compile(&dag).unwrap();
        assert_eq!(plan.step_count(), 3);

        // Verify topological order.
        let ids: Vec<&str> = plan.steps.iter().map(|s| s.node_id.as_str()).collect();
        let embed_pos = ids.iter().position(|&x| x == "embed").unwrap();
        let search_pos = ids.iter().position(|&x| x == "search").unwrap();
        let gen_pos = ids.iter().position(|&x| x == "generate").unwrap();
        assert!(embed_pos < search_pos);
        assert!(search_pos < gen_pos);

        // Each step in its own tier (linear = no parallelism).
        assert_eq!(plan.tier_count(), 3);
    }

    #[test]
    fn test_diamond_dag_parallel() {
        //     embed
        //    /     \
        // search  rerank
        //    \     /
        //    merge
        let dag = DagBuilder::new()
            .node("embed", embed())
            .node("search", search())
            .node("rerank", rerank())
            .node("merge", merge())
            .edge("embed", "search")
            .edge("embed", "rerank")
            .edge("search", "merge")
            .edge("rerank", "merge")
            .build()
            .unwrap();

        let plan = compile(&dag).unwrap();
        assert_eq!(plan.step_count(), 4);
        assert_eq!(plan.tier_count(), 3); // [embed], [search, rerank], [merge]

        // The middle tier should have 2 parallel steps.
        let mid = &plan.parallelizable[1];
        assert_eq!(mid.len(), 2);

        // Those parallel steps should reference each other.
        let s1 = &plan.steps[mid[0]];
        let s2 = &plan.steps[mid[1]];
        assert!(s1.can_parallel_with.contains(&s2.node_id));
        assert!(s2.can_parallel_with.contains(&s1.node_id));
    }

    #[test]
    fn test_transform_fusion() {
        // t1 -> t2 -> t3  (should fuse into single step)
        let dag = DagBuilder::new()
            .node("t1", transform("lowercase"))
            .node("t2", transform("trim"))
            .node("t3", transform("truncate"))
            .edge("t1", "t2")
            .edge("t2", "t3")
            .build()
            .unwrap();

        let plan = compile(&dag).unwrap();
        // All three fused into one step.
        assert_eq!(plan.step_count(), 1);
        if let PipelineNode::Transform { operation } = &plan.steps[0].node_type {
            assert!(operation.contains("lowercase"));
            assert!(operation.contains("trim"));
            assert!(operation.contains("truncate"));
        } else {
            panic!("expected fused Transform");
        }
    }

    #[test]
    fn test_redundant_cache_elimination() {
        // cache1 -> cache2 (same key) -> generate
        let dag = DagBuilder::new()
            .node("c1", cache("query"))
            .node("c2", cache("query"))
            .node("gen", generate())
            .edge("c1", "c2")
            .edge("c2", "gen")
            .build()
            .unwrap();

        let plan = compile(&dag).unwrap();
        // c2 should be eliminated.
        let ids: Vec<&str> = plan.steps.iter().map(|s| s.node_id.as_str()).collect();
        assert!(!ids.contains(&"c2"));
        assert!(ids.contains(&"c1"));
        assert!(ids.contains(&"gen"));
    }

    #[test]
    fn test_complex_multi_branch() {
        //       embed
        //      /  |  \
        //  search rerank filter
        //      \  |  /
        //       merge
        //         |
        //      generate
        let dag = DagBuilder::new()
            .node("embed", embed())
            .node("search", search())
            .node("rerank", rerank())
            .node("filter", filter())
            .node("merge", merge())
            .node("generate", generate())
            .edge("embed", "search")
            .edge("embed", "rerank")
            .edge("embed", "filter")
            .edge("search", "merge")
            .edge("rerank", "merge")
            .edge("filter", "merge")
            .edge("merge", "generate")
            .build()
            .unwrap();

        let plan = compile(&dag).unwrap();
        assert_eq!(plan.step_count(), 6);
        assert_eq!(plan.tier_count(), 4);

        // Tier 1 should have 3 parallel nodes.
        let tier1 = &plan.parallelizable[1];
        assert_eq!(tier1.len(), 3);
    }

    #[test]
    fn test_compile_correct_dependencies() {
        let dag = DagBuilder::new()
            .node("a", embed())
            .node("b", search())
            .edge("a", "b")
            .build()
            .unwrap();

        let plan = compile(&dag).unwrap();
        let b_step = plan.steps.iter().find(|s| s.node_id == "b").unwrap();
        assert_eq!(b_step.dependencies, vec!["a".to_string()]);
    }

    #[test]
    fn test_plan_serialization() {
        let dag = DagBuilder::new()
            .node("e", embed())
            .node("s", search())
            .edge("e", "s")
            .build()
            .unwrap();

        let plan = compile(&dag).unwrap();
        let json = serde_json::to_string(&plan).unwrap();
        let back: CompiledPlan = serde_json::from_str(&json).unwrap();
        assert_eq!(plan, back);
    }
}
