//! scheduling.

use crate::common::*;

// ---------------------------------------------------------------------------
// Production Scheduling (simple priority-based)
// ---------------------------------------------------------------------------

/// Priority level for scheduling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 0,
    Medium = 1,
    High = 2,
    Urgent = 3,
}

/// A schedulable production job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductionJob {
    pub work_order_id: Id,
    pub priority: Priority,
    pub duration_seconds: u64,
    pub earliest_start: Timestamp,
}

/// Simple forward scheduler: sorts by priority (desc) then earliest start.
#[must_use]
pub fn forward_schedule(
    jobs: &[ProductionJob],
    start_time: Timestamp,
) -> Vec<(Id, Timestamp, Timestamp)> {
    let mut sorted: Vec<_> = jobs.to_vec();
    sorted.sort_by(|a, b| {
        b.priority
            .cmp(&a.priority)
            .then(a.earliest_start.cmp(&b.earliest_start))
    });

    let mut current_time = start_time;
    sorted
        .iter()
        .map(|job| {
            let job_start = current_time.max(job.earliest_start);
            let job_end = job_start + job.duration_seconds;
            current_time = job_end;
            (job.work_order_id, job_start, job_end)
        })
        .collect()
}
