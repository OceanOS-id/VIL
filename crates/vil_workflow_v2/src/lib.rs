//! # vil_workflow_v2 (I07)
//!
//! AI Workflow Scheduler — DAG-based parallel task execution engine.
//!
//! Defines a task graph with dependencies, resolves execution order using
//! topological sort, and runs independent tasks in parallel via Tokio.

pub mod dag;
pub mod executor;
pub mod handlers;
pub mod pipeline_sse;
pub mod plugin;
pub mod scheduler;
pub mod semantic;
pub mod task;

pub use plugin::WorkflowPlugin;
pub use scheduler::{WorkflowResult, WorkflowScheduler};
pub use semantic::{WorkflowEvent, WorkflowFault, WorkflowFaultType, WorkflowState};
pub use task::{Task, TaskResult, TaskStatus, TaskType};

#[cfg(test)]
mod tests {
    use super::*;

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    #[test]
    fn test_single_task() {
        let sched = WorkflowScheduler::new();
        let tasks = vec![Task::new("t1", "embed", TaskType::Embed)];
        let result = rt().block_on(sched.submit(tasks)).unwrap();
        assert_eq!(result.results.len(), 1);
        assert_eq!(result.results[0].task_id, "t1");
        assert_eq!(result.results[0].status, TaskStatus::Success);
    }

    #[test]
    fn test_linear_tasks() {
        let sched = WorkflowScheduler::new();
        let tasks = vec![
            Task::new("t1", "embed", TaskType::Embed),
            Task::new("t2", "search", TaskType::Search).with_deps(vec!["t1".into()]),
            Task::new("t3", "generate", TaskType::Generate).with_deps(vec!["t2".into()]),
        ];
        let result = rt().block_on(sched.submit(tasks)).unwrap();
        assert_eq!(result.results.len(), 3);
        assert!(result.parallelism_ratio <= 1.01); // 3 tasks / 3 layers = 1.0
    }

    #[test]
    fn test_parallel_tasks() {
        let sched = WorkflowScheduler::new();
        let tasks = vec![
            Task::new("t1", "embed1", TaskType::Embed),
            Task::new("t2", "embed2", TaskType::Embed),
            Task::new("t3", "embed3", TaskType::Embed),
        ];
        let result = rt().block_on(sched.submit(tasks)).unwrap();
        assert_eq!(result.results.len(), 3);
        assert!(result.parallelism_ratio >= 2.9); // 3 tasks / 1 layer = 3.0
    }

    #[test]
    fn test_diamond_dependency() {
        // t1 -> t2, t3 -> t4
        let sched = WorkflowScheduler::new();
        let tasks = vec![
            Task::new("t1", "start", TaskType::Transform),
            Task::new("t2", "left", TaskType::Embed).with_deps(vec!["t1".into()]),
            Task::new("t3", "right", TaskType::Search).with_deps(vec!["t1".into()]),
            Task::new("t4", "merge", TaskType::Generate).with_deps(vec!["t2".into(), "t3".into()]),
        ];
        let result = rt().block_on(sched.submit(tasks)).unwrap();
        assert_eq!(result.results.len(), 4);
        // 3 layers: [t1], [t2,t3], [t4] -> 4/3 ≈ 1.33
        assert!(result.parallelism_ratio > 1.0);
    }

    #[test]
    fn test_timeout_task() {
        let sched = WorkflowScheduler::new();
        let tasks = vec![Task::new("t1", "slow", TaskType::Generate).with_timeout(1)];
        let result = rt().block_on(sched.submit(tasks)).unwrap();
        assert_eq!(result.results.len(), 1);
        assert_eq!(result.results[0].status, TaskStatus::Timeout);
    }

    #[test]
    fn test_empty_workflow() {
        let sched = WorkflowScheduler::new();
        let result = rt().block_on(sched.submit(vec![])).unwrap();
        assert_eq!(result.results.len(), 0);
        assert_eq!(result.total_ms, 0);
    }

    #[test]
    fn test_custom_task_type() {
        let sched = WorkflowScheduler::new();
        let tasks = vec![Task::new(
            "t1",
            "my-task",
            TaskType::Custom("my_plugin".into()),
        )];
        let result = rt().block_on(sched.submit(tasks)).unwrap();
        assert_eq!(result.results.len(), 1);
        assert_eq!(result.results[0].status, TaskStatus::Success);
    }

    #[test]
    fn test_dag_cycle_detection() {
        let tasks = vec![
            Task::new("a", "a", TaskType::Embed).with_deps(vec!["b".into()]),
            Task::new("b", "b", TaskType::Embed).with_deps(vec!["a".into()]),
        ];
        let result = dag::resolve_layers(&tasks);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Cycle"));
    }

    #[test]
    fn test_dag_missing_dep() {
        let tasks =
            vec![Task::new("a", "a", TaskType::Embed).with_deps(vec!["nonexistent".into()])];
        let result = dag::resolve_layers(&tasks);
        assert!(result.is_err());
    }
}
