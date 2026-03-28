//! Executor: run a `CompiledPlan`.

use std::collections::HashSet;

use crate::compiler::{CompiledPlan, ExecutionStep};
use vil_log::app_log;

/// The result of executing one step.
#[derive(Debug, Clone)]
pub struct StepResult {
    pub node_id: String,
    pub status: StepStatus,
}

/// Status of a completed step.
#[derive(Debug, Clone, PartialEq)]
pub enum StepStatus {
    /// Step completed successfully.
    Ok,
    /// Step was skipped (e.g. branch not taken).
    Skipped,
    /// Step failed with an error message.
    Failed(String),
}

/// Execution report for a full plan run.
#[derive(Debug, Clone)]
pub struct ExecutionReport {
    pub results: Vec<StepResult>,
    pub tiers_executed: usize,
}

impl ExecutionReport {
    /// Returns `true` if all steps succeeded (or were skipped).
    pub fn all_ok(&self) -> bool {
        self.results
            .iter()
            .all(|r| matches!(r.status, StepStatus::Ok | StepStatus::Skipped))
    }

    /// Number of failed steps.
    pub fn failed_count(&self) -> usize {
        self.results
            .iter()
            .filter(|r| matches!(r.status, StepStatus::Failed(_)))
            .count()
    }
}

/// A trait for user-provided step handlers.
pub trait StepHandler {
    /// Execute a single step. Return `Ok(())` on success, `Err(msg)` on failure.
    fn handle(&self, step: &ExecutionStep, completed: &HashSet<String>) -> Result<(), String>;
}

/// Default no-op handler that always succeeds (useful for dry runs / tests).
pub struct NoopHandler;

impl StepHandler for NoopHandler {
    fn handle(&self, _step: &ExecutionStep, _completed: &HashSet<String>) -> Result<(), String> {
        Ok(())
    }
}

/// Execute a compiled plan tier-by-tier.
///
/// Within each tier, steps are run sequentially (true async parallelism
/// requires a runtime — this executor provides the scheduling logic).
pub fn execute(plan: &CompiledPlan, handler: &dyn StepHandler) -> ExecutionReport {
    let mut results = Vec::new();
    let mut completed: HashSet<String> = HashSet::new();
    let mut tiers_executed = 0;

    for (tier_idx, tier) in plan.parallelizable.iter().enumerate() {
        app_log!(Debug, "executor_tier", { tier: tier_idx, steps: tier.len() });
        tiers_executed += 1;

        for &step_idx in tier {
            let step = &plan.steps[step_idx];

            // Verify all dependencies completed.
            let deps_met = step.dependencies.iter().all(|dep| completed.contains(dep));

            if !deps_met {
                results.push(StepResult {
                    node_id: step.node_id.clone(),
                    status: StepStatus::Skipped,
                });
                continue;
            }

            match handler.handle(step, &completed) {
                Ok(()) => {
                    app_log!(Info, "executor_step", { node_id: step.node_id.clone() });
                    completed.insert(step.node_id.clone());
                    results.push(StepResult {
                        node_id: step.node_id.clone(),
                        status: StepStatus::Ok,
                    });
                }
                Err(msg) => {
                    results.push(StepResult {
                        node_id: step.node_id.clone(),
                        status: StepStatus::Failed(msg),
                    });
                }
            }
        }
    }

    ExecutionReport {
        results,
        tiers_executed,
    }
}

/// Convenience: dry-run a plan (no-op handler).
pub fn dry_run(plan: &CompiledPlan) -> ExecutionReport {
    execute(plan, &NoopHandler)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::compile;
    use crate::dag::DagBuilder;
    use crate::node::PipelineNode;

    fn embed() -> PipelineNode {
        PipelineNode::Embed {
            model: "m".into(),
            dimensions: 128,
        }
    }
    fn search() -> PipelineNode {
        PipelineNode::Search {
            index: "i".into(),
            top_k: 5,
        }
    }

    #[test]
    fn test_dry_run_linear() {
        let dag = DagBuilder::new()
            .node("a", embed())
            .node("b", search())
            .edge("a", "b")
            .build()
            .unwrap();
        let plan = compile(&dag).unwrap();
        let report = dry_run(&plan);
        assert!(report.all_ok());
        assert_eq!(report.results.len(), 2);
        assert_eq!(report.tiers_executed, 2);
    }

    #[test]
    fn test_failing_handler() {
        struct FailHandler;
        impl StepHandler for FailHandler {
            fn handle(
                &self,
                step: &ExecutionStep,
                _completed: &HashSet<String>,
            ) -> Result<(), String> {
                if step.node_id == "b" {
                    Err("intentional failure".into())
                } else {
                    Ok(())
                }
            }
        }

        let dag = DagBuilder::new()
            .node("a", embed())
            .node("b", search())
            .edge("a", "b")
            .build()
            .unwrap();
        let plan = compile(&dag).unwrap();
        let report = execute(&plan, &FailHandler);
        assert!(!report.all_ok());
        assert_eq!(report.failed_count(), 1);
    }

    #[test]
    fn test_dry_run_empty() {
        let dag = DagBuilder::new().build().unwrap();
        let plan = compile(&dag).unwrap();
        let report = dry_run(&plan);
        assert!(report.all_ok());
        assert_eq!(report.results.len(), 0);
    }
}
