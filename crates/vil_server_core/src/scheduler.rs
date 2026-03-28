// =============================================================================
// VIL Server — Background Job Scheduler
// =============================================================================
//
// Schedule recurring (cron-like) and one-shot background tasks.
// Tasks run on the Tokio runtime alongside HTTP handlers.
//
// Usage:
//   scheduler.every(Duration::from_secs(60), "cleanup", || async {
//       cleanup_expired_sessions().await;
//   });
//   scheduler.once(Duration::from_secs(5), "warmup", || async {
//       warm_cache().await;
//   });

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tokio::task::JoinHandle;

/// Job status.
#[derive(Debug, Clone, Serialize)]
pub enum JobStatus {
    Scheduled,
    Running,
    Completed,
    Failed(String),
    Cancelled,
}

/// Job definition.
#[derive(Debug, Clone, Serialize)]
pub struct JobInfo {
    pub name: String,
    pub schedule: String,
    pub status: JobStatus,
    pub run_count: u64,
    pub last_run_duration_ms: Option<u64>,
}

/// Background job scheduler.
pub struct Scheduler {
    jobs: Arc<dashmap::DashMap<String, JobInfo>>,
    handles: HashMap<String, JoinHandle<()>>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(dashmap::DashMap::new()),
            handles: HashMap::new(),
        }
    }

    /// Schedule a recurring job.
    pub fn every<F, Fut>(&mut self, interval: Duration, name: &str, f: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send,
    {
        let jobs = self.jobs.clone();
        let job_name = name.to_string();

        jobs.insert(
            job_name.clone(),
            JobInfo {
                name: job_name.clone(),
                schedule: format!("every {}s", interval.as_secs()),
                status: JobStatus::Scheduled,
                run_count: 0,
                last_run_duration_ms: None,
            },
        );

        let handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;

                if let Some(mut info) = jobs.get_mut(&job_name) {
                    info.status = JobStatus::Running;
                }

                let start = std::time::Instant::now();
                f().await;
                let duration_ms = start.elapsed().as_millis() as u64;

                if let Some(mut info) = jobs.get_mut(&job_name) {
                    info.status = JobStatus::Scheduled;
                    info.run_count += 1;
                    info.last_run_duration_ms = Some(duration_ms);
                }
            }
        });

        self.handles.insert(name.to_string(), handle);
        {
            use vil_log::app_log;
            app_log!(Info, "scheduler.job.recurring", { job: name, interval_s: interval.as_secs() });
        }
    }

    /// Schedule a one-shot delayed job.
    pub fn once<F, Fut>(&mut self, delay: Duration, name: &str, f: F)
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send,
    {
        let jobs = self.jobs.clone();
        let job_name = name.to_string();

        jobs.insert(
            job_name.clone(),
            JobInfo {
                name: job_name.clone(),
                schedule: format!("once after {}s", delay.as_secs()),
                status: JobStatus::Scheduled,
                run_count: 0,
                last_run_duration_ms: None,
            },
        );

        let handle = tokio::spawn(async move {
            tokio::time::sleep(delay).await;

            if let Some(mut info) = jobs.get_mut(&job_name) {
                info.status = JobStatus::Running;
            }

            let start = std::time::Instant::now();
            f().await;
            let duration_ms = start.elapsed().as_millis() as u64;

            if let Some(mut info) = jobs.get_mut(&job_name) {
                info.status = JobStatus::Completed;
                info.run_count = 1;
                info.last_run_duration_ms = Some(duration_ms);
            }
        });

        self.handles.insert(name.to_string(), handle);
        {
            use vil_log::app_log;
            app_log!(Info, "scheduler.job.once", { job: name, delay_s: delay.as_secs() });
        }
    }

    /// Cancel a scheduled job.
    pub fn cancel(&mut self, name: &str) {
        if let Some(handle) = self.handles.remove(name) {
            handle.abort();
            if let Some(mut info) = self.jobs.get_mut(name) {
                info.status = JobStatus::Cancelled;
            }
            {
                use vil_log::app_log;
                app_log!(Info, "scheduler.job.cancelled", { job: name });
            }
        }
    }

    /// List all jobs.
    pub fn list_jobs(&self) -> Vec<JobInfo> {
        self.jobs.iter().map(|e| e.value().clone()).collect()
    }

    /// Get job count.
    pub fn job_count(&self) -> usize {
        self.jobs.len()
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}
